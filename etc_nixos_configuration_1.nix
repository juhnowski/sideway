# Edit this configuration file to define what should be installed on
# your system. Help is available in the configuration.nix(5) man page, on
# https://search.nixos.org/options and in the NixOS manual (`nixos-help`).

{ config, lib, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
      (fetchTarball "https://github.com/nix-community/nixos-vscode-server/tarball/master")
#      <home-manager/nixos>
 ];
    
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
  virtualisation.libvirtd.enable = true;
  networking.bridges.br0.interfaces = []; 

 
  # Use the systemd-boot EFI boot loader.
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.loader.timeout = 5;

  # Use latest kernel.
  # boot.kernelPackages = pkgs.linuxPackages_latest;

  boot.kernelModules = [
     "nbd"
     "nvmet" 
     "nvmet-tcp"
     "rdma_rxe"
     "ib_uverbs"
     "rdma_ucm"
  ];

  networking.hostName = "nixos"; # Define your hostname.

  # Configure network connections interactively with nmcli or nmtui.
  networking.networkmanager.enable = true;

  # Set your time zone.
   time.timeZone = "Europe/Moscow";

  # Configure network proxy if necessary
  # networking.proxy.default = "http://user:password@proxy:port/";
  # networking.proxy.noProxy = "127.0.0.1,localhost,internal.domain";

  # Select internationalisation properties.
  i18n.defaultLocale = "ru_RU.UTF-8";
  console = {
     font = "cyr-sun16";
     keyMap = "ru";
     earlySetup=true;
#     useXkbConfig = true; # use xkb.options in tty.
  };

  # Enable the X11 windowing system.
  # services.xserver.enable = true;


  

  # Configure keymap in X11
   services.xserver.xkb = {
     layout = "us,ru";
     options = "grp:alt_shift_toggle";
   };

  # Enable CUPS to print documents.
  # services.printing.enable = true;

  # Enable sound.
  # services.pulseaudio.enable = true;
  # OR
  # services.pipewire = {
  #   enable = true;
  #   pulse.enable = true;
  # };

  # Enable touchpad support (enabled default in most desktopManager).
  # services.libinput.enable = true;

  # Define a user account. Don't forget to set a password with ‘passwd’.
   users.users.ilya = {
     isNormalUser = true;
     extraGroups = [ "wheel" "networkmanager"]; # Enable ‘sudo’ for the user.
     packages = with pkgs; [
       tree
       mc
     ];
   };

  services.ollama = {
    enable = true;
    host = "0.0.0.0";
    # Если у вас есть GPU (NVIDIA), раскомментируйте строку ниже:
 #   acceleration = "cuda"; 
  };

  environment.systemPackages = with pkgs; [
	      mc
	      yazi
	      lynx
	      git
	      htop
	      curl
        helix
        curl
        vim
        plantuml
        jre
        ollama-cuda
        nbd
        fio
        wget
        rustc
        cargo
        rust-analyzer 
        clippy        
        rustfmt       
        pkg-config    
        openssl       
        gcc
        gnumake
        cmake        
        rdma-core
        libclang 
        ninja    
        python3  
        libnl.dev 
        libclang.dev 
  ];

  environment.variables = {
    COLORTERM = "truecolor";
    NIX_USER_PROFILE_DIR = "/nix/var/nix/profiles/per-user/$USER";
    EDITOR = "hx";
    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  };

  # programs.firefox.enable = true;

  # List packages installed in system profile.
  # You can use https://search.nixos.org/ to find more packages (and options).
  # environment.systemPackages = with pkgs; [
  #   vim # Do not forget to add an editor to edit configuration.nix! The Nano editor is also installed by default.
  #   wget
  # ];

  # Some programs need SUID wrappers, can be configured further or are
  # started in user sessions.
   programs.mtr.enable = true;
#   programs.gnupg.agent = {
#     enable = true;
#     enableSSHSupport = true;
#   };

  programs.ssh.startAgent = true;

  programs.nix-ld.enable = true;
    programs.nix-ld.libraries = with pkgs; [
       stdenv.cc.cc
       openssl
       curl
       zlib
       libz
       icu
       libgcc
       glibc
   ];

  # List services that you want to enable:

  # Enable the OpenSSH daemon.
  # Open ports in the firewall.
  networking.firewall.allowedTCPPorts = [ 
	  11434
	  22 
	  8080
	  80
    4420
  ];
  # networking.firewall.allowedUDPPorts = [ ... ];
  # Or disable the firewall altogether.
  networking.firewall.enable = true;


  services.openssh.enable = true;

  virtualisation.docker.enable = true;
  
  virtualisation.oci-containers.containers."plantuml-server" = {
    image = "plantuml/plantuml-server:jetty";
    ports = [ "8080:8080" ];
  };

  # Copy the NixOS configuration file and link it from the resulting system
  # (/run/current-system/configuration.nix). This is useful in case you
  # accidentally delete configuration.nix.
  # system.copySystemConfiguration = true;

  # This option defines the first version of NixOS you have installed on this particular machine,
  # and is used to maintain compatibility with application data (e.g. databases) created on older NixOS versions.
  #
  # Most users should NEVER change this value after the initial install, for any reason,
  # even if you've upgraded your system to a new NixOS release.
  #
  # This value does NOT affect the Nixpkgs version your packages and OS are pulled from,
  # so changing it will NOT upgrade your system - see https://nixos.org/manual/nixos/stable/#sec-upgrading for how
  # to actually do that.
  #
  # This value being lower than the current NixOS release does NOT mean your system is
  # out of date, out of support, or vulnerable.
  #
  # Do NOT change this value unless you have manually inspected all the changes it would make to your configuration,
  # and migrated your data accordingly.
  #
  # For more information, see `man configuration.nix` or https://nixos.org/manual/nixos/stable/options#opt-system.stateVersion .
  system.stateVersion = "25.11"; 

  
  nixpkgs.config.allowUnfree = true;
  # Драйверы NVIDIA
#  services.xserver.videoDrivers = [ "nvidia" ];
#  hardware.graphics.enable = true;
#  hardware.nvidia = {
#    modesetting.enable = true;
#    open = false; # Самый стабильный вариант для RTX 3070 на этой ветке
#  };
  
#  systemd.services.nvmet-target-setup = {
#    description = "NVMe-oF Target Setup";
#    after = [ "network.target" ];
#    wantedBy = [ "multi-user.target" ];
#  # Добавляем необходимые утилиты в PATH сервиса
#  path = with pkgs; [ 
#    util-linux    # для mount
#    coreutils     # для mkdir, echo, ln
#    kmod          # на случай если нужно modprobe
#  ];
#    serviceConfig = {
#      Type = "oneshot";
#      RemainAfterExit = true;
#    };
#script = ''
#  # Пути
#  S="/sys/kernel/config/nvmet/subsystems/nqn.2024-01.com.example:st01"
#  P="/sys/kernel/config/nvmet/ports/1"
#
#  # 1. Сначала всё отключаем, если оно было
#  if [ -L "$P/subsystems/nqn.2024-01.com.example:st01" ]; then
#    rm "$P/subsystems/nqn.2024-01.com.example:st01"
#  fi
#
#  # 2. Создаем структуру (mkdir -p не ругается если есть)
#  mkdir -p "$S/namespaces/10"
#  mkdir -p "$P/subsystems"
#
#  # 3. Сбрасываем Namespace (только если он включен)
#  echo 0 > "$S/namespaces/10/enable" || true
#  
#  # 4. Пишем конфиг
#  echo 1 > "$S/attr_allow_any_host"
#  echo -n /dev/nvme0n1 > "$S/namespaces/10/device_path"
#  echo 1 > "$S/namespaces/10/enable"
#
#  # 5. Конфиг порта
#  echo 192.168.3.27 > "$P/addr_traddr"
#  echo tcp > "$P/addr_trtype"
#  echo 4420 > "$P/addr_trsvcid"
#  echo ipv4 > "$P/addr_adrfam"
#
#  # 6. Линкуем обратно
#  ln -sfn "$S" "$P/subsystems/nqn.2024-01.com.example:st01"
#'';
#  };

  # 2. Настройки для вашего конкретного пользователя
  #   home-manager.users.ilya = { pkgs, ... }: {
  #     home.stateVersion = "24.11"; # Должна совпадать с версией системы
  # 
  #     # 3. Настройка Helix прямо здесь
  #     programs.helix = {
  #       enable = true;
  #       settings = {
  #         theme = "catppuccin_mocha";
  #         editor = {
  #           true-color = true;
  #           line-number = "relative";
  #         };
  #       };
  #     };
  #   };
}
