# Image Generation

With command:

```bash
scripts/image <id>
```

, this scripts generates an ISO image for a specific Raspberry Pi device using
the `image` script in the repository.

## Prerequisites

Before you start, make sure you have generated the secrets for the device using
the `mksecrets` script. The `image` script requires an encrypted secrets file
for the device.

## Usage

The `image` script takes one argument: `id`, which is the unique identifier for
the device. The script checks if an encrypted secrets file for this ID exists in
the `src/flake/enc` directory. If it does not, it exits with an error message.
