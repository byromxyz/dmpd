# dmpd

> [!WARNING]
> Experimental. Do not trust the output.

Accepts an MPD file and generates PNG and markdown (TODO) descriptions of the manifest.

## Install

### macOS

#### Automatic

The following will install `dmpd` to `usr/local/bin`. Requires sudo.

```bash
curl -s https://raw.githubusercontent.com/byromxyz/dmpd/main/scripts/install.sh | sudo bash -s
```

#### Manual

Download the relevant binary for your OS on the [Releases](https://github.com/byromxyz/dmpd/releases) page.

## Usage

Call with the path to a .mpd, .har, or a directory containing .mpd files.

```bash
dmpd ./manifest.mpd
dmpd ./requests.har
dmpd ./some/dir
```

### Config

- `--max-duration-ms XXX` Determines the maximum duration of the output. All other content will be trimmed.
- `--slice` Produces multiple PNG files for the whole manifest, each `max-duration-ms` long.
- `--from-ms XXX` Start point within the manifest.
- `--to-ms XXX` End point within the manifest.

## Example

![manifest](./examples/5-multi-period.png)
