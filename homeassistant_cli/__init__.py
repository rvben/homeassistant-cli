"""
homeassistant-cli: Agent-friendly CLI for Home Assistant.
"""

try:
    from importlib.metadata import version
    __version__ = version("ha-cli")
except ImportError:
    from importlib_metadata import version
    __version__ = version("ha-cli")
