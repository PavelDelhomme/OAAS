#!/usr/bin/env python3
"""
Worker IPC pour OAAS : compression des prompts via Microsoft LLMLingua.

Entrée / sortie : trames binaires (u32 big-endian = taille, puis JSON UTF-8).
Ops : init, compress

Dépendances : pip install -r scripts/requirements-llmlingua.txt
"""
from __future__ import annotations

import json
import struct
import sys
from typing import Any

COMPRESSOR = None


def read_frame() -> dict[str, Any] | None:
    hdr = sys.stdin.buffer.read(4)
    if not hdr or len(hdr) < 4:
        return None
    (n,) = struct.unpack("!I", hdr)
    raw = sys.stdin.buffer.read(n)
    if len(raw) != n:
        return None
    return json.loads(raw.decode("utf-8"))


def write_frame(obj: dict[str, Any]) -> None:
    b = json.dumps(obj, ensure_ascii=False).encode("utf-8")
    if len(b) > 0xFFFFFFFF:
        raise RuntimeError("payload trop grand")
    sys.stdout.buffer.write(struct.pack("!I", len(b)))
    sys.stdout.buffer.write(b)
    sys.stdout.buffer.flush()


def handle_init(req: dict[str, Any]) -> None:
    global COMPRESSOR
    from llmlingua import PromptCompressor

    model_name = req.get("model_name") or "microsoft/phi-2"
    use_llmlingua2 = bool(req.get("use_llmlingua2"))
    device_map = req.get("device_map") or "cpu"

    kwargs: dict[str, Any] = {"model_name": model_name, "device_map": device_map}
    if use_llmlingua2:
        kwargs["use_llmlingua2"] = True

    COMPRESSOR = PromptCompressor(**kwargs)
    write_frame({"ok": True})


def message_text(m: dict[str, Any]) -> str | None:
    c = m.get("content")
    if isinstance(c, str):
        return c
    if isinstance(c, list):
        parts: list[str] = []
        for it in c:
            if isinstance(it, dict) and it.get("type") == "text":
                parts.append(str(it.get("text") or ""))
        if parts:
            return "\n".join(parts)
        return None
    return None


def compress_chat(
    body: dict[str, Any], rate: float, target_token: int
) -> tuple[dict[str, Any] | None, str | None]:
    global COMPRESSOR
    if COMPRESSOR is None:
        return None, "compressor non initialisé"

    messages = body.get("messages")
    if not isinstance(messages, list) or not messages:
        return body, None

    lines: list[str] = []
    for m in messages:
        if not isinstance(m, dict):
            return body, None
        role = str(m.get("role", ""))
        text = message_text(m)
        if text is None:
            # contenu multimodal ou non texte : on ne modifie pas la requête
            return body, None
        lines.append(f"### {role}\n{text}\n")

    full = "\n".join(lines)
    kwargs: dict[str, Any] = {"instruction": "", "question": "", "rate": float(rate)}
    if target_token > 0:
        kwargs["target_token"] = int(target_token)

    try:
        out = COMPRESSOR.compress_prompt(full, **kwargs)
    except Exception as e:
        return None, str(e)

    comp = out.get("compressed_prompt")
    if not isinstance(comp, str):
        return None, "réponse compress_prompt inattendue"

    new_body = dict(body)
    new_body["messages"] = [
        {
            "role": "system",
            "content": (
                "(Contexte compressé par OAAS via LLMLingua — conserve le sens et les contraintes)\n"
                + comp
            ),
        }
    ]
    return new_body, None


def main() -> None:
    while True:
        req = read_frame()
        if req is None:
            break
        op = req.get("op")
        try:
            if op == "init":
                handle_init(req)
            elif op == "compress":
                body = req.get("body")
                if not isinstance(body, dict):
                    write_frame({"ok": False, "error": "body doit être un objet JSON"})
                    continue
                rate = float(req.get("rate", 0.5))
                tt = int(req.get("target_token", 0))
                new_body, err = compress_chat(body, rate, tt)
                if err:
                    write_frame({"ok": False, "error": err})
                else:
                    write_frame({"ok": True, "body": new_body})
            else:
                write_frame({"ok": False, "error": f"op inconnue: {op!r}"})
        except Exception as e:
            write_frame({"ok": False, "error": repr(e)})


if __name__ == "__main__":
    main()
