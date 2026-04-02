"""
homeassistant-cli: Agent-friendly CLI for Home Assistant.
"""

try:
    from importlib.metadata import version
    __version__ = version("homeassistantcli")
except ImportError:
    from importlib_metadata import version
    __version__ = version("homeassistantcli")
