"""The ``saturnix-filter`` command-line tool.

This module depends on the ``cli`` extra (click, rich, pillow). It is meant for
experimenting and is not required for the production runtime.
"""

from __future__ import annotations

import sys

_CLI_EXTRA_HINT = (
    "The 'saturnix-filter' command-line tool needs extra packages.\n"
    'Install them with:  pip install "saturnix-filter[cli]"'
)


def _cli_extra_available() -> bool:
    try:
        import click  # noqa: F401
        import rich  # noqa: F401
        from PIL import Image  # noqa: F401
    except ImportError:
        return False
    return True


def _build_cli():
    """Construct and return the click command group.

    All click/rich imports live here so the module stays importable without the
    ``cli`` extra.
    """
    import difflib
    from pathlib import Path

    import click
    from rich.console import Console
    from rich.table import Table

    from . import apply_film_inplace, available_filters
    from ._filters import describe
    from .convert import convert_file, default_output_path

    console = Console()
    err_console = Console(stderr=True)

    def resolve_filter(name: str) -> str:
        filters = available_filters()
        if name in filters:
            return name
        lower = {f.lower(): f for f in filters}
        if name.lower() in lower:
            return lower[name.lower()]
        suggestion = difflib.get_close_matches(name, filters, n=1)
        hint = f" Did you mean '{suggestion[0]}'?" if suggestion else ""
        raise click.ClickException(
            f"Unknown filter: '{name}'.{hint} "
            "Run 'saturnix-filter list' to see all filters."
        )

    @click.group()
    @click.version_option(package_name="saturnix-filter", message="%(version)s")
    def cli() -> None:
        """Apply Saturnix film-simulation filters to images."""

    @cli.command("list")
    def list_filters() -> None:
        """List all available filters with their descriptions."""
        table = Table(title="Available filters")
        table.add_column("Filter", style="bold cyan", no_wrap=True)
        table.add_column("Description")
        for name in available_filters():
            table.add_row(name, describe(name))
        console.print(table)

    @cli.command()
    @click.argument(
        "inputs",
        nargs=-1,
        required=True,
        type=click.Path(exists=True, dir_okay=False, path_type=Path),
    )
    @click.option(
        "-f",
        "--filter",
        "filters",
        multiple=True,
        required=True,
        help="Filter to apply (repeatable to render several looks).",
    )
    @click.option(
        "-o",
        "--output",
        type=click.Path(dir_okay=False, path_type=Path),
        help="Output file. Only valid with a single input and single filter; "
        "otherwise names are generated automatically next to each input.",
    )
    @click.option(
        "-q", "--quality", default=90, show_default=True, help="JPEG quality."
    )
    @click.option(
        "-w",
        "--max-width",
        default=0,
        show_default=True,
        help="Downscale output to this width (0 = keep original size).",
    )
    def convert(inputs, filters, output, quality, max_width) -> None:
        """Convert one or more IMAGES with one or more filters.

        Examples:

          saturnix-filter convert photo.jpg -f S-Gold

          saturnix-filter convert *.jpg -f S-Gold -f S-Halation -w 1200
        """
        resolved = [resolve_filter(f) for f in filters]

        if output is not None and (len(inputs) > 1 or len(resolved) > 1):
            raise click.ClickException(
                "--output can only be used with a single input image and a "
                "single filter. Omit it to auto-name outputs for batches."
            )

        jobs = [(src, name) for src in inputs for name in resolved]
        with click.progressbar(jobs, label="Converting", show_pos=True) as bar:
            for src, name in bar:
                dst = output if output is not None else default_output_path(src, name)
                try:
                    convert_file(src, dst, name, quality=quality, max_width=max_width)
                except Exception as exc:
                    raise click.ClickException(
                        f"Failed to convert '{src}' with '{name}': {exc}"
                    )
        console.print(f"[green]Done.[/] Wrote {len(jobs)} image(s).")

    @cli.command()
    @click.argument(
        "image",
        type=click.Path(exists=True, dir_okay=False, path_type=Path),
    )
    @click.option(
        "--out-dir",
        "out_dir",
        type=click.Path(file_okay=False, path_type=Path),
        help="Directory to write the outputs to (default: next to the source image).",
    )
    @click.option(
        "--width",
        default=480,
        show_default=True,
        help="Thumbnail width for the gallery (0 = keep original size).",
    )
    @click.option("--quality", default=88, show_default=True, help="JPEG quality.")
    def gallery(image, out_dir, width, quality) -> None:
        """Apply every filter to IMAGE, rendering a gallery.

        Writes a downscaled <stem>_original.jpg plus <stem>_<filter>.jpg for
        each available filter, where <stem> is IMAGE's file name.
        """
        from PIL import Image

        directory = out_dir if out_dir is not None else image.parent
        directory.mkdir(parents=True, exist_ok=True)
        stem = image.stem
        filters = available_filters()

        # Downscaled (unfiltered) original for the gallery.
        original_out = directory / f"{stem}_original.jpg"
        with Image.open(image) as img:
            rgb = img.convert("RGB")
            w, h = rgb.size
            if width and w > width:
                rgb = rgb.resize((width, round(h * width / w)), Image.LANCZOS)
            rgb.save(original_out, quality=quality)
        console.print(f"[green][ok][/]  {original_out.name}")

        for name in filters:
            out = directory / f"{stem}_{name.lower()}.jpg"
            convert_file(image, out, name, quality=quality, max_width=width)
            console.print(f"[green][ok][/]  {out.name}")

    # Representative default benchmark set: one profile per code path
    # (monochrome, channel-mix, vignette+grain, light-leak, halation postpass)
    # plus the VHS path.
    _DEFAULT_BENCH_FILTERS = (
        "S-MonoX",
        "S-Vivid",
        "S-Gold",
        "S-Leak",
        "S-Halation",
        "VHS",
    )

    @cli.command()
    @click.option(
        "-f",
        "--filter",
        "filters",
        multiple=True,
        help="Filter to benchmark (repeatable). Default: a representative set.",
    )
    @click.option(
        "--width", default=4656, show_default=True, help="Benchmark image width."
    )
    @click.option(
        "--height", default=3496, show_default=True, help="Benchmark image height."
    )
    @click.option(
        "--repeats",
        default=5,
        show_default=True,
        help="Timed repetitions per filter (median is reported).",
    )
    def bench(filters, width, height, repeats) -> None:
        """Benchmark filter throughput on a synthetic in-memory image.

        Reports the median (and min/max) time per filter over REPEATS runs,
        after one warm-up pass. Measures pure processing (no file I/O), matching
        the numbers quoted in the README. Runs on the target hardware too.
        """
        import statistics
        import time

        names = list(filters) if filters else list(_DEFAULT_BENCH_FILTERS)
        resolved = [resolve_filter(n) for n in names]

        pixels = width * height
        # A deterministic, non-uniform buffer so no code path is trivially
        # optimised away.
        base = bytes((i * 7 + 13) & 0xFF for i in range(768))
        template = bytearray((base * ((pixels * 3) // 768 + 1))[: pixels * 3])

        table = Table(title=f"Benchmark  {width}x{height}  ({repeats} repeats)")
        table.add_column("Filter", style="bold cyan", no_wrap=True)
        table.add_column("Median", justify="right")
        table.add_column("Min", justify="right")
        table.add_column("Max", justify="right")

        for name in resolved:
            # Warm-up (not timed).
            buf = bytearray(template)
            apply_film_inplace(buf, width, height, name)

            samples = []
            for _ in range(repeats):
                buf = bytearray(template)
                start = time.perf_counter()
                apply_film_inplace(buf, width, height, name)
                samples.append((time.perf_counter() - start) * 1000.0)

            table.add_row(
                name,
                f"{statistics.median(samples):.1f} ms",
                f"{min(samples):.1f} ms",
                f"{max(samples):.1f} ms",
            )
            console.print(f"[dim]benchmarked {name}[/]")

        console.print(table)

    return cli


def main() -> None:
    """Entry point registered as the ``saturnix-filter`` console script."""
    if not _cli_extra_available():
        print(_CLI_EXTRA_HINT, file=sys.stderr)
        raise SystemExit(1)
    cli = _build_cli()
    cli()


if __name__ == "__main__":
    main()
