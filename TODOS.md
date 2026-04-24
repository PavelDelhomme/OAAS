# OAAS — TODOS (checklist)

Liste **actionnable** au jour le jour. Le détail et les investigations : **[BACKLOG.md](BACKLOG.md)**. La vision / l’état produit : **[STATUS.md](STATUS.md)**.

---

## Poste de travail / conflit ressources

- [ ] Comprendre pourquoi **Ollama** redémarre seul : suivre la section *Ollama qui se lance tout seul* dans [BACKLOG.md](BACKLOG.md) (systemd user, autostart, etc.).
- [ ] Après diagnostic : `systemctl --user disable ollama` (ou équivalent) si tu veux **zéro** Ollama en arrière-plan pendant les sessions OAAS-only.

---

## Tests manuels (qualité)

- [ ] Machine **NVIDIA** : section GPU dans `/oaas/system.json` (nom, VRAM, %).
- [ ] Machine **AMD ROCm** : `rocm-smi --json` puis valider l’UI / le parse.
- [ ] **Continue** : uniquement `config.json` → fusion → backup + fichier à jour.
- [ ] **Continue** : `config.yaml` + `config.json` → écriture YAML prioritaire.
- [ ] `oaas status` sans serveur puis **avec** `make serve` (TCP + HTTP OK).
- [ ] `make status-ci` après changements Rust.
- [ ] `make install-user` puis `oaas status` depuis `$HOME` (PATH).
- [x] `cargo test` (helpers ROCm-like, `/proc`, `classify_exe`).

---

## Développement (extraits du backlog)

- [ ] Parse ROCm : exemples JSON terrain 5.x / 6.x.
- [ ] Workspace `.continue/config.yaml`.
- [ ] RAG + index doc (moyen terme).
- [ ] Exemple `oaas.service` + doc systemd.
