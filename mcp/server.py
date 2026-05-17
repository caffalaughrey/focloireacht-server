#!/usr/bin/env python3
"""MCP server wrapping focloireacht-server terminology API."""
import os
import httpx
from mcp.server.fastmcp import FastMCP

BASE_URL = os.environ.get("FOCLOIREACHT_URL", "http://localhost:5005").rstrip("/")

mcp = FastMCP("focloireacht")


@mcp.tool()
def search_domains(q: str, limit: int = 10) -> list:
    """Search Irish terminology domains by keyword.
    Returns domain labels with term counts — use this to discover
    the right domain name before calling get_vocab.
    """
    r = httpx.get(f"{BASE_URL}/term/domains/search", params={"q": q, "limit": limit}, timeout=10)
    r.raise_for_status()
    return r.json()["domains"]


@mcp.tool()
def get_vocab(domain: str, limit: int = 200) -> dict:
    """Load the full Irish/English vocabulary for a terminology domain.
    Returns ga/en term pairs with POS tags.
    Use search_domains first to find the exact domain label.
    """
    r = httpx.get(f"{BASE_URL}/term/vocab", params={"domain": domain, "limit": limit}, timeout=10)
    r.raise_for_status()
    return r.json()


@mcp.tool()
def translate_en2ga(term: str, domain: str = None, limit: int = 5) -> list:
    """Look up the Irish translation(s) of an English term.
    Optionally filter by domain label for domain-specific senses.
    """
    params: dict = {"term": term, "limit": limit}
    if domain:
        params["domain"] = domain
    r = httpx.get(f"{BASE_URL}/term/en2ga", params=params, timeout=10)
    r.raise_for_status()
    return r.json()["matches"]


@mcp.tool()
def translate_ga2en(term: str, domain: str = None, limit: int = 5) -> list:
    """Look up the English translation(s) of an Irish term.
    Optionally filter by domain label for domain-specific senses.
    """
    params: dict = {"term": term, "limit": limit}
    if domain:
        params["domain"] = domain
    r = httpx.get(f"{BASE_URL}/term/ga2en", params=params, timeout=10)
    r.raise_for_status()
    return r.json()["matches"]


if __name__ == "__main__":
    mcp.run()
