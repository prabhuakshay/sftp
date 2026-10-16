# sftp brand

<p>
  <img src="svg/lockup.svg" alt="sftp" height="64">
</p>

The mark is `~/`, the shell's home path. It stands for opening a remote machine's filesystem from your own terminal. The tile color is an oxide orange, a nod to Rust.

## Colors

| Name         | Hex       | Use                                         |
| ------------ | --------- | ------------------------------------------- |
| Oxide        | `#C24E1C` | Primary. Logo tile on light backgrounds.    |
| Oxide Bright | `#E3693A` | Logo tile and accents on dark backgrounds.  |
| Shell        | `#1A2025` | Text, dark backgrounds, mono logo.          |
| Slate        | `#5B6670` | Secondary text on light.                    |
| Fog          | `#97A3AD` | Secondary text on dark.                     |
| Mist         | `#ECEFF1` | Light backgrounds, wordmark on dark.        |
| Paper        | `#FFFFFF` | The `~/` glyph on any tile.                 |

White on Oxide has a contrast ratio of 4.8:1.

## Files

| File                          | What it is                                                    |
| ----------------------------- | ------------------------------------------------------------- |
| `svg/logo.svg`                | Primary mark. Use this by default.                            |
| `svg/logo-on-dark.svg`        | Brighter tile for dark backgrounds.                           |
| `svg/logo-mono-dark.svg`      | Single color, Shell tile.                                     |
| `svg/logo-mono-light.svg`     | Single color, white tile.                                     |
| `svg/favicon.svg`             | Heavier strokes for 48 px and below.                          |
| `svg/glyph*.svg`              | `~/` without the tile, in Oxide, Shell and white.             |
| `svg/wordmark*.svg`           | "sftp" alone, for light and dark backgrounds.                 |
| `svg/lockup*.svg`             | Mark and wordmark side by side, for light and dark.           |
| `svg/social-preview.svg`      | Source for the GitHub social preview.                         |
| `png/logo-{16…1024}.png`      | Raster mark. Sizes 48 px and below use the favicon cut.       |
| `png/lockup*.png`             | Raster lockups, 256 px tall.                                  |
| `png/apple-touch-icon.png`    | 180 px square with no rounded corners. iOS rounds it.         |
| `png/social-preview.png`      | 1280 × 640. Upload under repo Settings → Social preview.      |
| `icon.ico`                    | Windows icon: 16, 32, 48 and 256 px.                           |

The wordmark is drawn as strokes, not set in a font, so it renders the same everywhere. The only text set in a font is the tagline in `social-preview.svg`. It uses Consolas, so use the PNG wherever that font may be missing.

## Usage

- Leave clear space around the mark of at least a quarter of the tile's width.
- Don't use the mark smaller than 16 px. Below 48 px, use `favicon.svg` or the small PNGs.
- Keep the glyph white on the colored tiles. Don't recolor, stretch, rotate or outline the mark.
- In a lockup the wordmark sits to the right of the mark, centered on its x-height. Don't stack them or change the spacing.
