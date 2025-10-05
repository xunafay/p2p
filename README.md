starting the relay: cargo run -p relay -- --port 8080 --secret-key-seed 0 --key <swarm_secret_key>
starting the client: cargo run -p client -- --relay-address /ip4/<relay-ip>/tcp/8080 --relay-peer-id <relay-peer-id> --key <swarm-secret-key>
