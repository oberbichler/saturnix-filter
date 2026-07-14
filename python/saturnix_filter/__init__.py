"""Saturnix film-simulation filters.

The core API is implemented in Rust and exposed here:

- ``apply_film_inplace(buffer, width, height, filter_name)`` applies a filter
  to a raw RGB byte buffer in place.
- ``available_filters()`` returns the list of every filter name.

The core has no Python dependencies. The optional ``saturnix-filter``
command-line tool (see :mod:`saturnix_filter.cli`) requires the ``cli`` extra:

    pip install "saturnix-filter[cli]"
"""

from .saturnix_filter import apply_film_inplace, available_filters

__all__ = ["apply_film_inplace", "available_filters"]
