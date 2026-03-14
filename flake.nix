{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: 
  let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    commonModule = { config, pkgs, ... }: {
      system.stateVersion = "24.11";
      boot.kernelModules = [ "rdma_rxe" "ib_uverbs" ];
      
      environment.systemPackages = with pkgs; [ 
        rdma-core 
        iproute2 
        pciutils 
        ethtool 
        pkg-config 
        clang 
        libclang.lib
        rustc 
        cargo
        # --- ДОБАВЛЕНО: Инструменты Cargo ---
        just
        cargo-nextest
        cargo-llvm-cov
        # ------------------------------------
        cmake 
        ninja 
        gcc
        gnumake
        python3
        libnl.dev
      ];

      environment.variables = {
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        CC = "clang";
        # Путь к драйверам ibverbs, иначе ошибка (код 95, EOPNOTSUPP) 
        RDMAV_FORCED_CONFIG_FILE = "${pkgs.rdma-core}/etc/libibverbs.d";
        LD_LIBRARY_PATH = "${pkgs.rdma-core}/lib";
      };

      # --- ДОБАВЛЕНО: Монтирование virtiofs ---
      fileSystems."/src" = {
        device = "host_share"; # Метка (tag) из QEMU
        fsType = "virtiofs";
        options = [ "defaults" ];
      };

      services.udev.extraRules = ''
        ACTION=="add", SUBSYSTEM=="net", NAME=="eth1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="init-rdma.service"
      '';

      systemd.services.init-rdma = {
        description = "Initialize Soft-RoCE Link on eth1";
        bindsTo = [ "sys-subsystem-net-devices-eth1.device" ];
        after = [ "sys-subsystem-net-devices-eth1.device" ];
        path = [ pkgs.rdma-core pkgs.iproute2 ];
        script = ''
          if ! rdma link show rxe0 >/dev/null 2>&1; then
            rdma link add rxe0 type rxe netdev eth1 || true
          fi
        '';
        serviceConfig = { Type = "oneshot"; RemainAfterExit = true; };
      };

      networking.firewall.enable = false;
      services.getty.autologinUser = "root";
    };

  in {
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [ pkg-config clang rustc cargo ];
      buildInputs = [ pkgs.rdma-core ];
      LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    };

    nixosConfigurations = {
      node1 = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          commonModule
          ({ ... }: {
            networking.hostName = "node1";
            networking.interfaces.eth1.ipv4.addresses = [{ address = "10.0.0.1"; prefixLength = 24; }];
            virtualisation.vmVariant.virtualisation.qemu.options = [ 
              # --- Настройки памяти для virtiofs ---
              "-m 4096"
              "-object memory-backend-memfd,id=mem,size=4G,share=on"
              "-numa node,memdev=mem"
              # --- Сокет для vhost-user-fs ---
              "-chardev socket,id=char0,path=/tmp/vfs-node1.sock"
              "-device vhost-user-fs-pci,queue-size=1024,chardev=char0,tag=host_share"
              # Сеть
              "-netdev socket,id=n1,listen=:1234 -device virtio-net-pci,netdev=n1,mac=52:54:00:12:34:01" 
            ];
          })
        ];
      };

      node2 = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          commonModule
          ({ ... }: {
            networking.hostName = "node2";
            networking.interfaces.eth1.ipv4.addresses = [{ address = "10.0.0.2"; prefixLength = 24; }];
            virtualisation.vmVariant.virtualisation.qemu.options = [ 
              "-m 4096"
              "-object memory-backend-memfd,id=mem,size=4G,share=on"
              "-numa node,memdev=mem"
              "-chardev socket,id=char0,path=/tmp/vfs-node2.sock"
              "-device vhost-user-fs-pci,queue-size=1024,chardev=char0,tag=host_share"
              # Сеть
              "-netdev socket,id=n1,connect=127.0.0.1:1234 -device virtio-net-pci,netdev=n1,mac=52:54:00:12:34:02" 
            ];
          })
        ];
      };
    };
  };
}
