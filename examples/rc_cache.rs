//! InfiniBand Reliable Connection (RC) Cache
//!
//! Этот модуль реализует глобальный кэш, используя протокол Reliable Connection (RC) от InfiniBand.
//! Словарь
//! wc (Work Completions) - завершение работы в CQ
//! wr (Work Request) - запрос на выполнение RDMA
//! Completion Queue (CQ) - очередь завершения операций
//! gid - глобальный идентификатор для IB-порта
//! SGE (Segment Descriptor)
//! l-key - локальный ключ доступа к памяти
//! r-key - удаленный ключ доступа к памяти
//! Protection Domain (PD)

use std::io::{Error, Read, Write};
use std::net::{IpAddr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use sideway::ibverbs::address::{AddressHandleAttribute, Gid};
use sideway::ibverbs::completion::{
    CreateCompletionQueueWorkCompletionFlags, GenericCompletionQueue, WorkCompletionStatus,
};
use sideway::ibverbs::device::{DeviceInfo, DeviceList};
use sideway::ibverbs::device_context::Mtu;
use sideway::ibverbs::queue_pair::{
    PostSendGuard, QueuePair, QueuePairAttribute, QueuePairState, SetScatterGatherEntry, WorkRequestFlags,
};
use sideway::ibverbs::AccessFlags;

use byte_unit::{Byte, UnitType};

const SEND_WR_ID: u64 = 0;
const RECV_WR_ID: u64 = 1;

const CHUNK_SIZE: usize = 4096;

#[derive(Debug, Parser)]
#[clap(name = "rc_cache", version = "0.1.0")]
pub struct Args {
    /// Прослушивать порт / Подключиться к порту
    #[clap(long, short = 'p', default_value_t = 18515)]
    port: u16,
    /// Устройство IB для использования
    #[clap(long, short = 'd')]
    ib_dev: Option<String>,
    /// Порт устройства IB
    #[clap(long, short = 'i', default_value_t = 1)]
    ib_port: u8,
    /// Размер сообщения для обмена
    #[clap(long, short = 's', default_value_t = CHUNK_SIZE)]
    size: u32,
    /// MTU (Maximum Transmission Unit) – предельно большой объем информации, который можно единовременно направить через сетевой интерфейс.
    #[clap(long, short = 'm', value_enum, default_value_t = PathMtu(Mtu::Mtu1024))]
    mtu: PathMtu,
    /// Количество сообщений, которые можно отправить за один раз.
    #[clap(long, short = 'r', default_value_t = 500)]
    rx_depth: u32,
    /// Глубина приёмного буфера
    #[clap(long, short = 'n', default_value_t = 500)]
    iter: u32,
    /// Service level value
    #[clap(long, short = 'l', default_value_t = 0)]
    sl: u8,
    /// Local port GID index
    #[clap(long, short = 'g', default_value_t = 0)]
    gid_idx: u8,
    /// Get CQE with timestamp
    #[arg(long, short = 't', default_value_t = false)]
    ts: bool,
    /// If no value provided, start a server and wait for connection, otherwise, connect to server at [host]
    #[arg(name = "host")]
    server_ip: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct PathMtu(Mtu);

impl ValueEnum for PathMtu {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self(Mtu::Mtu256),
            Self(Mtu::Mtu512),
            Self(Mtu::Mtu1024),
            Self(Mtu::Mtu2048),
            Self(Mtu::Mtu4096),
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self.0 {
            Mtu::Mtu256 => Some(clap::builder::PossibleValue::new("256")),
            Mtu::Mtu512 => Some(clap::builder::PossibleValue::new("512")),
            Mtu::Mtu1024 => Some(clap::builder::PossibleValue::new("1024")),
            Mtu::Mtu2048 => Some(clap::builder::PossibleValue::new("2048")),
            Mtu::Mtu4096 => Some(clap::builder::PossibleValue::new("4096")),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct PingPongDestination {
    lid: u32,
    qp_number: u32,
    packet_seq_number: u32,
    gid: Gid,
}

#[allow(clippy::while_let_on_iterator)] // отключает предупреждение линтера о нерекомендуемом использовании while let на итераторах.
fn main() -> anyhow::Result<()> {
    // ---- Парсинг аргументов и инициализация счётчиков  ----
    let args = Args::parse(); // парсит аргументы командной строки
    let mut scnt: u32 = 0; // счётчики для отправленных пакетов
    let mut rcnt: u32 = 0; // счётчики для полученных пакетов
    let rx_depth = if args.iter > args.rx_depth {
        //глубина приёмного буфера (сколько запросов на приём можно держать в очереди)
        args.rx_depth
    } else {
        args.iter
    };
    let mut rout: u32 = 0; // количество активных приёмных запросов

    // ---- Получение списка устройств InfiniBand ----
    let device_list = DeviceList::new().expect("Failed to get IB devices list"); // Получает список IB-устройств.
    let device = match args.ib_dev {
        //Если указано имя устройства (args.ib_dev), ищет его, иначе берёт первое доступное.
        Some(ib_dev) => device_list
            .iter()
            .find(|dev| dev.name().eq(&ib_dev))
            .unwrap_or_else(|| panic!("IB device {ib_dev} not found")),
        None => device_list.iter().next().expect("No IB device found"),
    };

    // ---- Открытие контекста устройства ----
    let context = device
        .open()
        .unwrap_or_else(|_| panic!("Couldn't get context for {}", device.name()));

    // ---- Выделение Protection Domain (PD) - необходим для управления доступом к памяти
    let pd = context.alloc_pd().unwrap_or_else(|_| panic!("Couldn't allocate PD"));

    // ---- Регистрация памяти для отправки ----
    let send_data: Vec<u8> = vec![0; args.size as _]; // выделяет в куче(heap) память размером args.size и заполняет нулями
    let send_mr = unsafe {
        pd.reg_mr(
            // Memory Region, регистрирует буфер для доступа через IB
            send_data.as_ptr() as _,
            send_data.len(),
            AccessFlags::LocalWrite | AccessFlags::RemoteWrite,
        )
        .unwrap_or_else(|_| panic!("Couldn't register send MR"))
    };

    // ---- Выделение памяти для приема ----
    let mut recv_data: Vec<u8> = vec![0; args.size as _]; // выделяет в куче(heap) память размером args.size и заполняет нулями
    let recv_mr = unsafe {
        pd.reg_mr(
            recv_data.as_ptr() as _,
            recv_data.len(),
            AccessFlags::LocalWrite | AccessFlags::RemoteWrite,
        )
        .unwrap_or_else(|_| panic!("Couldn't register recv MR"))
    };

    // ---- Получение GID и PSN для отправки ----
    let gid = context.query_gid(args.ib_port, args.gid_idx.into()).unwrap(); // глобальный идентификатор для IB-порта
    let psn = rand::random::<u32>() & 0xFFFFFF; // случайный номер последовательности пакетов для уникальности

    // ---- Создание очереди событий - Completion Queue (CQ) ----
    let mut cq_builder = context.create_cq_builder();

    cq_builder.setup_wc_flags(CreateCompletionQueueWorkCompletionFlags::StandardFlags);

    let cq = cq_builder.setup_cqe(rx_depth + 1).build_ex().unwrap(); // очередь завершения операций, куда будут приходить уведомления о завершении отправки/приёма с rx_depth + 1 элементами

    let cq_handle = GenericCompletionQueue::from(Arc::clone(&cq));

    // ---- Создание очереди передачи - Queue Pair (QP)  ----

    let mut builder = pd.create_qp_builder();

    let mut qp = builder // создание QP (пара очередей) с максимальным встроенным размером данных 128 байт, связанным с CQ для отправки и приёма
        .setup_max_inline_data(128) // максимальный размер сообщения, которое может быть отправлено непосредственно в очередь отправки (в запросе на выполнение RDMA).
        .setup_send_cq(cq_handle.clone()) // очередь завершения операций для отправки
        .setup_recv_cq(cq_handle) // очередь завершения операций для приёма
        .setup_max_send_wr(1) // максимальное количество запросов на отправку
        .setup_max_recv_wr(rx_depth) // максимальное количество запросов на приём
        .build_ex()
        .unwrap_or_else(|_| panic!("Couldn't create QP"));

    // ---- Настройка атрибутов QP и перевод QP в состояние Init ----
    let mut attr = QueuePairAttribute::new();
    attr.setup_state(QueuePairState::Init)
        .setup_pkey_index(0) // Partition Key Index для QP - Индекс ключа раздела
        .setup_port(args.ib_port)
        .setup_access_flags(AccessFlags::LocalWrite | AccessFlags::RemoteWrite);
    qp.modify(&attr)?;

    // ---- Постинг приёмных запросов:Заполняет приёмную очередь запросами на приём данных. ----
    for _i in 0..rx_depth {
        let mut guard = qp.start_post_recv(); // Запускает операцию приема данных после получения, при этом каждая пара очередей должна содержать одновременно только один PostRecvGuard.

        let recv_handle = guard.construct_wr(RECV_WR_ID); // Создает новый объект RecvWorkRequestHandle для настройки нового запроса на выполнение RDMA; каждая пара очередей должна содержать только один объект RecvWorkRequestHandle одновременно.

        // -- Каждый запрос регистрируется с указанием буфера и его размера.
        unsafe {
            recv_handle.setup_sge(recv_mr.lkey(), recv_data.as_mut_ptr() as _, args.size);
            // Устанавливает SGE (Segment Descriptor) для приема данных, указывая ключ доступа к памяти (lkey) и указатель на буфер для записи.
        };

        guard.post().unwrap();
    }

    rout += rx_depth; // количество активных приёмных запросов

    println!(" local address: QPN {:#06x}, PSN {psn:#08x}, GID {gid}", qp.qp_number());

    // ---- Подключение к серверу или ожидание подключения от клиента  ----
    let mut stream = match args.server_ip {
        Some(ref ip_str) => {
            let ip = IpAddr::from_str(ip_str).expect("Invalid IP address");
            let server_addr = SocketAddr::from((ip, args.port));
            TcpStream::connect(server_addr)?
        },
        None => {
            let server_addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, args.port));
            let listener = TcpListener::bind(server_addr)?;
            let (stream, _peer_addr) = listener.accept()?;
            stream
        },
    };

    // ---- Отправка и получение контекста через TCP  ----
    let send_context = |stream: &mut TcpStream, dest: &PingPongDestination| {
        let msg_buf = to_allocvec(dest).unwrap();
        let size = msg_buf.len().to_be_bytes();
        stream.write_all(&size)?;
        stream.write_all(&msg_buf)?;
        stream.flush()?;

        Ok::<(), Error>(())
    };

    let recv_context = |stream: &mut TcpStream, msg_buf: &mut Vec<u8>| {
        let mut size = usize::to_be_bytes(0);
        stream.read_exact(&mut size)?;
        msg_buf.clear();
        msg_buf.resize(usize::from_be_bytes(size), 0);
        stream.read_exact(&mut *msg_buf)?;
        let dest: PingPongDestination = from_bytes(msg_buf).unwrap();

        Ok::<PingPongDestination, Error>(dest)
    };

    let local_context = PingPongDestination {
        lid: 1,
        qp_number: qp.qp_number(),
        packet_seq_number: psn,
        gid,
    };
    let mut msg_buf = Vec::new();
    let _ = send_context(&mut stream, &local_context);
    let remote_context = recv_context(&mut stream, &mut msg_buf)?;
    println!(
        "remote address: QPN {:#06x}, PSN {:#08x}, GID {}",
        remote_context.qp_number, remote_context.packet_seq_number, remote_context.gid
    );

    // -- Переводит QP в состояние ReadyToReceive, настраивает параметры пути, номер удалённой QP, PSN
    let mut attr = QueuePairAttribute::new();
    attr.setup_state(QueuePairState::ReadyToReceive)
        .setup_path_mtu(args.mtu.0)
        .setup_dest_qp_num(remote_context.qp_number)
        .setup_rq_psn(psn)
        .setup_max_dest_read_atomic(0)
        .setup_min_rnr_timer(0);

    // setup address vector
    let mut ah_attr = AddressHandleAttribute::new();

    ah_attr
        .setup_dest_lid(1)
        .setup_port(args.ib_port)
        .setup_service_level(args.sl)
        .setup_grh_src_gid_index(args.gid_idx)
        .setup_grh_dest_gid(&remote_context.gid)
        .setup_grh_hop_limit(1);
    attr.setup_address_vector(&ah_attr);
    qp.modify(&attr)?;

    let mut attr = QueuePairAttribute::new();
    attr.setup_state(QueuePairState::ReadyToSend)
        .setup_sq_psn(remote_context.packet_seq_number)
        .setup_timeout(12)
        .setup_retry_cnt(7)
        .setup_rnr_retry(7)
        .setup_max_read_atomic(0);

    qp.modify(&attr)?;

    let mut outstanding_send = false;

    if args.server_ip.is_some() {
        let mut guard = qp.start_post_send();

        let send_handle = guard.construct_wr(SEND_WR_ID, WorkRequestFlags::Signaled).setup_send();

        unsafe {
            send_handle.setup_sge(send_mr.lkey(), send_data.as_ptr() as _, args.size);
        }

        guard.post()?;
        outstanding_send = true;
    }
    // Основной цикл опроса CQ и обработки завершённых операций
    {
        loop {
            match cq.start_poll() {
                Ok(mut poller) => {
                    while let Some(wc) = poller.next() {
                        if wc.status() != WorkCompletionStatus::Success as u32 {
                            panic!(
                                "Failed status {:#?} ({}) for wr_id {}",
                                Into::<WorkCompletionStatus>::into(wc.status()),
                                wc.status(),
                                wc.wr_id()
                            );
                        }
                        match wc.wr_id() {
                            SEND_WR_ID => {
                                scnt += 1;
                                outstanding_send = false;
                            },
                            RECV_WR_ID => {
                                rcnt += 1;
                                rout -= 1;

                                // Post more receives if the receive side credit is low
                                if rout <= rx_depth / 2 {
                                    let to_post = rx_depth - rout;
                                    for _ in 0..to_post {
                                        let mut guard = qp.start_post_recv();
                                        let recv_handle = guard.construct_wr(RECV_WR_ID);
                                        unsafe {
                                            recv_handle.setup_sge(
                                                recv_mr.lkey(),
                                                recv_data.as_mut_ptr() as _,
                                                args.size,
                                            );
                                        };
                                        guard.post().unwrap();
                                    }
                                    rout += to_post;
                                }
                            },
                            _ => {
                                panic!("Unknown error!");
                            },
                        }

                        if scnt < args.iter && !outstanding_send {
                            // Постинг запросов на отправку (post_send)
                            let mut guard = qp.start_post_send();
                            let send_handle = guard.construct_wr(SEND_WR_ID, WorkRequestFlags::Signaled).setup_send();
                            unsafe {
                                send_handle.setup_sge(send_mr.lkey(), send_data.as_ptr() as _, args.size);
                                // запись данных в SGE - это как адрес посылки
                            }
                            guard.post()?;
                            outstanding_send = true;
                        }
                    }
                },
                Err(_) => {
                    continue;
                },
            }

            // Check if we're done
            if scnt >= args.iter && rcnt >= args.iter {
                break;
            }
        }
    }

    Ok(())
}

//--------------------------------------------------------------------------------

use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq)]
pub struct CacheEntry {
    pub id: u64,
    pub payload: Vec<u8>,
    pub origin_id: u32,
    pub timestamp: u128, // Время создания записи в наносекундах
}

pub struct RdmaCache {
    pub cache: DashMap<u64, CacheEntry>,
    tx_send: Sender<CacheEntry>,
    local_node_id: u32,
}

impl RdmaCache {
    pub fn new(local_node_id: u32, tx_send: Sender<CacheEntry>) -> Self {
        Self {
            cache: DashMap::new(),
            tx_send,
            local_node_id,
        }
    }

    fn get_now() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    }

    /// Локальная вставка: всегда обновляем и рассылаем
    pub fn insert(&self, key: u64, payload: Vec<u8>) {
        let entry = CacheEntry {
            id: key,
            payload,
            origin_id: self.local_node_id,
            timestamp: Self::get_now(),
        };

        self.cache.insert(key, entry.clone());
        let _ = self.tx_send.send(entry);
    }

    /// Локальная вставка: записывает и рассылает ТОЛЬКО если ключа еще нет
    pub fn insert_if_absent(&self, key: u64, payload: Vec<u8>) {
        // Проверяем наличие ключа и вставляем атомарно, если его нет
        let mut was_inserted = false;

        self.cache.entry(key).or_insert_with(|| {
            was_inserted = true;
            CacheEntry {
                id: key,
                payload,
                origin_id: self.local_node_id,
                timestamp: Self::get_now(),
            }
        });

        // Рассылаем в сеть только если запись действительно была создана сейчас
        if was_inserted {
            if let Some(entry) = self.cache.get(&key) {
                let _ = self.tx_send.send(entry.clone());
            }
        }
    }

    /// Обработка входящих данных с разрешением конфликтов
    pub fn process_incoming(&self, remote_entry: CacheEntry) {
        // 1. Фильтрация эха
        if remote_entry.origin_id == self.local_node_id {
            return;
        }

        // 2. Атомарная проверка через entry-API DashMap
        self.cache
            .entry(remote_entry.id)
            .and_modify(|local_val| {
                // Если пришедшие данные новее — обновляем
                if remote_entry.timestamp > local_val.timestamp {
                    *local_val = remote_entry.clone();
                }
                // Если таймстампы равны (редко), побеждает узел с бóльшим ID
                else if remote_entry.timestamp == local_val.timestamp && remote_entry.origin_id > local_val.origin_id
                {
                    *local_val = remote_entry.clone();
                }
            })
            .or_insert(remote_entry); // Если ключа нет — просто вставляем
    }
}

// два подхода:
// 1. Один раз положили в DashMap и не обновляем insert_if_absent
// 2. Обновляем при каждом входящем запросе
