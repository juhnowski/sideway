# Запуск стенда

# Терминал 1
Для Node 1:
```bash
cd ~/sideway
sudo nix run nixpkgs#virtiofsd -- \
  --socket-path=/tmp/vfs-node1.sock \
  --shared-dir=$(pwd) \
  --announce-submounts \
  --sandbox none
```

# Терминал 2
Для Node 2:
```bash
cd ~/sideway
sudo nix run nixpkgs#virtiofsd -- \
  --socket-path=/tmp/vfs-node2.sock \
  --shared-dir=$(pwd) \
  --announce-submounts \
  --sandbox none
  ```


# Терминал 3
```bash
cd ~/sideway
nix build .#nixosConfigurations.node1.config.system.build.vm
sudo QEMU_OPTS="-nographic" nix run .#nixosConfigurations.node1.config.system.build.vm
```


# Терминал 4
```bash
cd ~/sideway
nix build .#nixosConfigurations.node2.config.system.build.vm
sudo QEMU_OPTS="-nographic" nix run .#nixosConfigurations.node2.config.system.build.vm
```

# монтируем вручную на двух виртуалках
```bash
mkdir -p /src
mount -t virtiofs host_share /src
cd /src
cargo build --release
cargo build --examples
```

# Очистить неудачный билд
```bash
rm -rf target
rm Cargo.lock
```

# Проверяем статус RDMA ссылок
```bash
rdma link
```
# Проверяем наличие символьных устройств
```bash
ls -l /dev/infiniband/
```

# Если не поднят интерфейс
```bash
ip link set eth1 up
```

# show_gids
```bash
cargo run --example show_gids
 Dev  | Port | Index |                   GID                   |   IPv4   |  Ver   | Netdev 
------+------+-------+-----------------------------------------+----------+--------+--------
 rxe0 |  1   |   0   | fe80:0000:0000:0000:5054:00ff:fe12:3401 |          | RoCEv2 |  eth1  
 rxe0 |  1   |   1   | 0000:0000:0000:0000:0000:ffff:0a00:0001 | 10.0.0.1 | RoCEv2 |  eth1 
```