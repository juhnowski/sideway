# Deploying the stand
This guide will help you get started successfully with the project – creating a stand on two virtual machines.

## Prerequisites:
  - NixOS on host. Dirty example configuration - `etc_nixos_configuration.nix`. You may merge in with yours own condig in `/etc/nixos/configuration.nix`

# Therminal 1
For Node 1:
```bash
cd ~/sideway
sudo nix run nixpkgs#virtiofsd -- \
  --socket-path=/tmp/vfs-node1.sock \
  --shared-dir=$(pwd) \
  --announce-submounts \
  --sandbox none
```

# Therminal 2
For Node 2:
```bash
cd ~/sideway
sudo nix run nixpkgs#virtiofsd -- \
  --socket-path=/tmp/vfs-node2.sock \
  --shared-dir=$(pwd) \
  --announce-submounts \
  --sandbox none
  ```


# Therminal 3
Make sure the program is running in Terminal 1.
```bash
cd ~/sideway
nix build .#nixosConfigurations.node1.config.system.build.vm
sudo QEMU_OPTS="-nographic" nix run .#nixosConfigurations.node1.config.system.build.vm
```


# Therminal 4
Make sure the program is running in Terminal 2.
```bash
cd ~/sideway
nix build .#nixosConfigurations.node2.config.system.build.vm
sudo QEMU_OPTS="-nographic" nix run .#nixosConfigurations.node2.config.system.build.vm
```

# Therminal 3,4: We mount it manually on two virtual machines.
```bash
mkdir -p /src
mount -t virtiofs host_share /src
cd /src
cargo build --release
cargo build --examples
```

# Therminal 3,4: Clear a failed build (if necessary)
```bash
rm -rf target
rm Cargo.lock
```

# Therminal 3,4: Checking the status of RDMA links
```bash
rdma link
```
# Therminal 3,4: Checking for the presence of character devices
```bash
ls -l /dev/infiniband/
```

# Therminal 3,4: If the interface is not raised
```bash
ip link set eth1 up
```

# Therminal 3 or 4: show_gids
```bash
cargo run --example show_gids
 Dev  | Port | Index |                   GID                   |   IPv4   |  Ver   | Netdev 
------+------+-------+-----------------------------------------+----------+--------+--------
 rxe0 |  1   |   0   | fe80:0000:0000:0000:5054:00ff:fe12:3401 |          | RoCEv2 |  eth1  
 rxe0 |  1   |   1   | 0000:0000:0000:0000:0000:ffff:0a00:0001 | 10.0.0.1 | RoCEv2 |  eth1 
```

# Run example rc_pingpong
## Therminal 3 
```bash
[root@node1:/src]# cargo run --example rc_pingpong -- -d rxe0 -i 1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/examples/rc_pingpong -d rxe0 -i 1`
 local address: QPN 0x0012, PSN 0xb701eb, GID fe80:0000:0000:0000:5054:00ff:fe12:3401
remote address: QPN 0x0012, PSN 0x8873de, GID fe80:0000:0000:0000:5054:00ff:fe12:3402
2048000 bytes in 0.12 seconds = 16.80 MiB/s
1000 iters in 0.12 seconds = 116.23µs/iter
```

## Therminal 4 
```bash
[root@node2:/src]# cargo run --example rc_pingpong -- -d rxe0 -i 1 10.0.0.1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/examples/rc_pingpong -d rxe0 -i 1 10.0.0.1`
 local address: QPN 0x0012, PSN 0x8873de, GID fe80:0000:0000:0000:5054:00ff:fe12:3402
remote address: QPN 0x0012, PSN 0xb701eb, GID fe80:0000:0000:0000:5054:00ff:fe12:3401
2048000 bytes in 0.12 seconds = 16.80 MiB/s
1000 iters in 0.12 seconds = 116.29µs/iter
```

# Run example cmtime
## Therminal 3 
```bash
cargo run --example cmtime -- -b 0.0.0.0 -p 18515 -c 10
```

## Therminal 4 
### Connections = 10
```bash
[root@node2:/src]# cargo run --example cmtime -- -s 10.0.0.1 -c 10
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/examples/cmtime -s 10.0.0.1 -c 10`
 Step            | Total (ms) | Max (us) | Min (us) 
-----------------+------------+----------+----------
 CreateId        |       5.33 |  5303.44 |     0.89 
 ResolveAddr     |       0.12 |    70.59 |    22.74 
 ResolveRoute    |       0.08 |    29.10 |     6.65 
 CreateQueuePair |       0.19 |    58.79 |     9.60 
 Connect         |       1.16 |   988.98 |   701.28 
 Disconnect      |      40.46 | 40448.38 |   160.27 
 ```
### Connections = 10000
```bash
[root@node2:/src]# cargo run --example cmtime -- -s 10.0.0.1 -c 10000
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/examples/cmtime -s 10.0.0.1 -c 10000`
 Step            | Total (ms) |  Max (us) | Min (us) 
-----------------+------------+-----------+----------
 CreateId        |      34.34 |   5281.40 |     0.82 
 ResolveAddr     |      44.10 |   1625.26 |     5.06 
 ResolveRoute    |      31.78 |   2746.60 |     6.99 
 CreateQueuePair |     244.56 |    118.66 |     8.48 
 Connect         |     986.88 | 900415.51 |   847.53 
 Disconnect      |     134.05 |  96991.11 |  1551.19 
 ```

 # Run example ibv_devinfo
```bash
cargo run --example
```

# Run rc_cache
## Therminal 3
```bash
cargo run --example rc_cache -- -d rxe0 -i 1
```
## Therminal 4
```bash
cargo run --example rc_cache -- -d rxe0 -i 1 10.0.0.1
```
