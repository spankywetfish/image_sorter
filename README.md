# image_sorter

Recursively scans a folder for image files and sorts them into
`<output>/<YYYY>/<MM>/<DD>/` subdirectories using EXIF date metadata, falling back to file creation date where EXIF data is not available.

## Supported formats

JPEG, TIFF, HEIC/HEIF, WebP, PNG, and raw formats (CR2, CR3, NEF, ARW, ORF, RW2, DNG, RAF).

> **Note:** PNG files rarely carry EXIF data; those will be reported as "no EXIF date".

## Build

```bash
cargo build --release
# binary is at: target/release/image_sorter
```

## Usage

```
image_sorter [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input   <INPUT>   Source folder to scan
  -o, --output  <OUTPUT>  Destination root folder
  -c, --copy              Copy files instead of moving them
  -d, --dry-run           Preview actions without touching files
  -h, --help              Print help
  -V, --version           Print version
```

## Examples

**Preview what would happen (safe — no files moved):**
```bash
image_sorter --input ~/Pictures/unsorted --output ~/Pictures/sorted --dry-run
```

**Move files into sorted folders:**
```bash
image_sorter --input ~/Pictures/unsorted --output ~/Pictures/sorted
```

**Copy instead of move:**
```bash
image_sorter --input /Volumes/Camera --output ~/Pictures/sorted --copy
```

## Output structure

```
sorted/
├── 2023/
│   ├── 07/
│   │   ├── 14/
│   │   │   ├── IMG_0001.jpg
│   │   │   └── IMG_0002.jpg
│   │   └── 15/
│   │       └── DSC_0042.nef
│   └── 12/
│       └── 25/
│           └── photo.heic
└── 2024/
    └── 01/
        └── 01/
            └── IMG_0100.jpg
```

## Notes

- EXIF tag priority: `DateTimeOriginal` → `DateTimeDigitized` → `DateTime`
- Duplicate filenames in the same destination folder get a `_1`, `_2`, … suffix
- Files with no EXIF date data are reported but left in place
- Works across filesystems (falls back to copy+delete when needed)

Most of the code was originally generated using claude, however it did not work.
It took a bit of finagling to get a functioning program, after which the fall back to using file dates and colourised output was added.
 
