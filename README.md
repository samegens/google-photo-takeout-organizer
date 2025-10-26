# organize-photo-zip

CLI tool to organize Google Photos Takeout ZIP exports into date-based folders.

Built for my personal use case: I keep all my photos organized in a `YYYY/YYYY-MM-DD/` structure.
All photos from my DSLRs are automatically imported into this structure, but I missed
all photos taken with my mobile phone. With this tool I can make Google Photo Takeout zips and
import them into this structure. Somehow I imported some Lightroom exported photos into Google Photos.
Since these files are already in my desired structure, I want them skipped automatically.

## How I use Google Takeout

- Log in to your Google account.
- Go to your [Google Dashboard](https://myaccount.google.com/dashboard?pli=1).
- Under Photos click on Download.
- Under 'Create a new export' wait for 'All photo albums included' to appear.
- Click 'All photo albums included'.
- Click 'Deselect all' and select the year you want to download and click OK.
- Click 'Next step'.
- Leave type, frequency and destination as is, click 'Create export'.
- Wait for the export to finish, this will take a while.
- When it's done, download the zip.

## Features

- **Date-based organization**: Photos organized into `YYYY/YYYY-MM-DD/` structure
- **Smart date extraction**: Uses EXIF metadata first, falls back to filename patterns
- **Intelligent filtering**: Automatically skips duplicates from DSLR cameras, Lightroom, and Google-generated files
- **Orphaned edit handling**: Keeps edited photos when originals are missing, skips them when originals exist
- **Flat structure**: Preserves just the filename, removes Google Takeout's nested paths

## Installation

```bash
cargo install --path .
```

## Usage

**Basic usage** (with filtering):
```bash
organize-photo-zip --input takeout.zip --output ./photos
```

**Organize all files** (no filtering):
```bash
organize-photo-zip --input takeout.zip --output ./photos --no-filter
```

## What Gets Filtered

By default, the tool skips:
- Photos from Nikon DSLR cameras (detected via EXIF)
- Lightroom-processed photos (detected via EXIF)
- Google-generated `-MIX` files
- Google-edited photos when the original exists

Use `--no-filter` to organize everything.

## Example

**Input ZIP structure:**
```
Takeout/Google Photos/Photos from 2014/
├── DSC_9157.JPG (Nikon DSLR)
├── IMG-20150108-WA0000.jpg
├── 2014-09-29.jpg
└── photo-edited.jpg (original exists)
```

**Output:**
```
organized_photos/
├── 2014/
│   └── 2014-09-29/
│       └── 2014-09-29.jpg
└── 2015/
    └── 2015-01-08/
        └── IMG-20150108-WA0000.jpg
```

Filtered out: `DSC_9157.JPG` (DSLR), `photo-edited.jpg` (original exists)

## Supported Date Formats

- EXIF DateTimeOriginal field (preferred)
- Filename patterns: `YYYY-MM-DD`, `YYYYMMDD_HHMMSS`, `IMG-YYYYMMDD`, `IMG_YYYYMMDD_HHMMSS`

## License

MIT
