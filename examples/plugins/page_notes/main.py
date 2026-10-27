"""Example external plugin of OpenECraft (protocol: docs/plugins.md).

Reads one JSON request from stdin and writes one JSON answer to stdout.
"""

import json
import sys

DESCRIPTION = {
    "commands": [
        {
            "name": "set_page_description",
            "help": "Sets the description of a page",
            "params": [
                {"name": "page", "kind": "str", "help": "Page name, e.g. 3"},
                {"name": "description", "kind": "str", "help": "New description"},
            ],
        }
    ],
    "checks": [{"name": "page_without_description", "title": "Pages without description"}],
    "reports": [{"name": "page_notes", "title": "Page notes"}],
    "exporters": [{"name": "page_notes_md", "title": "Page notes (Markdown)", "extension": "md"}],
}


def answer(request):
    method = request["method"]
    if method == "describe":
        return DESCRIPTION
    pages = request["project"].get("pages", [])
    if method == "command":
        params = request["params"]
        for page in pages:
            if page.get("name") == params["page"]:
                page["description"] = params["description"]
                return {"result": "ok", "project": request["project"]}
        return {"error": "Unknown page: " + params["page"]}
    if method == "check":
        return {
            "issues": [
                {"severity": "warning", "message": "Page {} has no description".format(p.get("name")), "page": i, "item": 0}
                for i, p in enumerate(pages)
                if not p.get("description", "").strip()
            ]
        }
    if method == "report":
        return {"headers": ["Page", "Description"], "rows": [[p.get("name", ""), p.get("description", "")] for p in pages]}
    if method == "export":
        lines = ["# Pages", ""] + ["- {}: {}".format(p.get("name", ""), p.get("description", "")) for p in pages]
        return {"content": "\n".join(lines) + "\n"}
    return {"error": "Unknown method: " + method}


if __name__ == "__main__":
    # The JSON is UTF-8 whatever the system's code page (Windows would garble
    # non-ASCII characters of the project otherwise).
    request = json.loads(sys.stdin.buffer.read().decode("utf-8"))
    sys.stdout.buffer.write(json.dumps(answer(request), ensure_ascii=False).encode("utf-8"))
