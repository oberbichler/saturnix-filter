"""Tests for the `saturnix-filter` command-line tool.

These exercise the CLI extra (click, rich, pillow); they are skipped if the
extra is not installed, mirroring the graceful-degradation design.
"""

import pytest
from PIL import Image

import saturnix_filter

click = pytest.importorskip("click")
from click.testing import CliRunner  # noqa: E402

from saturnix_filter.cli import _build_cli  # noqa: E402


@pytest.fixture
def cli():
    return _build_cli()


@pytest.fixture
def runner():
    return CliRunner()


def _write_image(path, size=(48, 32)):
    Image.new("RGB", size, (120, 90, 60)).save(path)


def test_available_filters_matches_rust_export():
    # The CLI's single source of truth for names is the Rust export.
    names = saturnix_filter.available_filters()
    assert "S-Gold" in names
    assert "VHS" in names
    assert len(names) == len(set(names)), "filter names must be unique"


def test_list_shows_filters(cli, runner):
    result = runner.invoke(cli, ["list"])
    assert result.exit_code == 0
    assert "S-Gold" in result.output
    assert "Kodak Gold" in result.output


def test_convert_auto_names_output(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src)
    result = runner.invoke(cli, ["convert", str(src), "-f", "S-Gold"])
    assert result.exit_code == 0, result.output
    out = tmp_path / "photo_s-gold.jpg"
    assert out.exists()


def test_convert_explicit_output(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src)
    dst = tmp_path / "result.jpg"
    result = runner.invoke(
        cli, ["convert", str(src), "-f", "S-Noir", "-o", str(dst)]
    )
    assert result.exit_code == 0, result.output
    assert dst.exists()


def test_convert_multiple_filters_batch(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src)
    result = runner.invoke(
        cli, ["convert", str(src), "-f", "S-Gold", "-f", "S-Halation"]
    )
    assert result.exit_code == 0, result.output
    assert (tmp_path / "photo_s-gold.jpg").exists()
    assert (tmp_path / "photo_s-halation.jpg").exists()


def test_convert_unknown_filter_suggests(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src)
    result = runner.invoke(cli, ["convert", str(src), "-f", "S-Golld"])
    assert result.exit_code != 0
    assert "Unknown filter" in result.output
    assert "S-Gold" in result.output  # suggestion


def test_convert_case_insensitive_filter(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src)
    result = runner.invoke(cli, ["convert", str(src), "-f", "s-gold"])
    assert result.exit_code == 0, result.output
    assert (tmp_path / "photo_s-gold.jpg").exists()


def test_convert_output_rejected_for_batch(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src)
    dst = tmp_path / "out.jpg"
    result = runner.invoke(
        cli,
        ["convert", str(src), "-f", "S-Gold", "-f", "S-Noir", "-o", str(dst)],
    )
    assert result.exit_code != 0
    assert "--output" in result.output


def test_gallery_renders_all_filters(cli, runner, tmp_path):
    src = tmp_path / "s_01.jpg"
    _write_image(src, size=(80, 60))
    result = runner.invoke(cli, ["gallery", str(src), "--width", "40"])
    assert result.exit_code == 0, result.output
    assert (tmp_path / "s_01_original.jpg").exists()
    for name in saturnix_filter.available_filters():
        assert (tmp_path / f"s_01_{name.lower()}.jpg").exists(), name


def test_gallery_requires_image_argument(cli, runner):
    result = runner.invoke(cli, ["gallery"])
    assert result.exit_code != 0


def test_gallery_out_dir(cli, runner, tmp_path):
    src = tmp_path / "photo.jpg"
    _write_image(src, size=(80, 60))
    out = tmp_path / "rendered"
    result = runner.invoke(
        cli, ["gallery", str(src), "--out-dir", str(out), "--width", "40"]
    )
    assert result.exit_code == 0, result.output
    assert (out / "photo_original.jpg").exists()
    assert (out / "photo_s-gold.jpg").exists()


def test_bench_runs_and_reports_timings(cli, runner):
    # Small size and few repeats keep the test fast; it must still report a
    # per-filter timing and exit cleanly.
    result = runner.invoke(
        cli,
        ["bench", "--width", "64", "--height", "48", "--repeats", "2", "-f", "S-Gold"],
    )
    assert result.exit_code == 0, result.output
    assert "S-Gold" in result.output
    # A millisecond figure should appear in the output.
    assert "ms" in result.output


def test_bench_unknown_filter_suggests(cli, runner):
    result = runner.invoke(cli, ["bench", "-f", "S-Nope"])
    assert result.exit_code != 0
    assert "Unknown filter" in result.output
