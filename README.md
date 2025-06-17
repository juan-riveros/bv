# bucket viewer

`bucket-viewer` (bv) is a webapp for viewing object storage as a paginated gallery. 

## Installation

```bash
git clone https://github.com/juan-riveros/bv
cargo build --release
cp target/release/bv $HOME/.local/bin/
```

## Usage

```bash
> bv
Config { page_size: 32, addr: "0.0.0.0", port: 8099, buckets: "camera-2023,camera-2024,camera-2025", region: "homelab", endpoint: "https://buckets.homelab.lan", access_key_id: "user", secret_access_key: REDACTED }
2025-06-17T04:24:11.795024Z  INFO bv: 97148 items found in bucket camera-2023
2025-06-17T04:24:12.303899Z  INFO bv: 41784 items found in bucket camera-2024
2025-06-17T04:24:12.830544Z  INFO bv: 30079 items found in bucket camera-2025
```

## Contributing

Open to PRs, but expect no support or assurances of stability.

## License

[MIT](https://choosealicense.com/licenses/mit/)
