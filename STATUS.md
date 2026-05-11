# OAAS — suivi projet (STATUS)

Document **vivant** : **vision** et **état des fonctionnalités** (tableau ci‑dessous).  
**Checklist courte** : [TODOS.md](TODOS.md) · **backlog détaillé + investigations** (ex. Ollama qui redémarre seul) : [BACKLOG.md](BACKLOG.md) · **usage** : [README.md](README.md).

---

## Vision cible (ce que je veux vraiment à la fin)

1. **Orchestrateur local** fiable : `llama-server` + proxy **OpenAI-compatible** + profils YAML, **sans Ollama**.
2. **Expérience développeur** : `make` / `oaas` clairs (`status`, `doctor`, `serve`, `dev`), install user ou système, archive `dist`.
3. **Interface** : tableau de bord `/oaas/` (catalogue, profils, Continue, métriques machine).
4. **Continue / IDE** : fusion **YAML ou JSON**, sauvegarde avant écriture, ouverture projet dans `code` / `cursor` / `codium`.
5. **Observabilité** : RAM / charge / **CPU par cœur** / **NVIDIA** + **AMD ROCm** (quand outils présents), RSS OAAS + llama.
6. **Suite** (pas encore livré) : index local + **RAG** sur cache doc, miroirs téléchargement, alignement versions outils (`go version`, etc.).
7. **MemePalace** (voir [BACKLOG.md](BACKLOG.md)) : exploiter la **technique du palais de la mémoire** (*method of loci* / méthode des lieux) comme axe produit — structuration mentale des infos, possible lien avec RAG, docs ou parcours dans l’UI ; **pas encore implémenté** dans le code.

---

## État actuel (fonctionnel / partiel / absent)

| Domaine | État | Détail |
|--------|------|--------|
| Proxy `/v1/*` | **OK** | Chat, models, streaming ; LLMLingua optionnel. |
| Catalogue modèles + `pull` | **OK** | YAML + CLI + UI ; reprise téléchargement via `.part` + `Range`. |
| Docs `oaas docs` | **Partiel** | git/fetch/devdocs-notes ; pas de RAG. |
| UI `/oaas/` | **OK** | Statut, catalogue, IDE, système, navigation. |
| `oaas status` / `make status` | **OK** | Config, GGUF, TCP, HTTP `/oaas/`, Continue, binaire courant ; `--ci` = fmt+clippy+build. |
| `GET /oaas/workstation.json` | **OK** | Même synthèse pendant `serve` (profil actif du serveur). |
| Métriques `/oaas/system.json` | **OK (Linux)** | Charge, RAM, RSS, **CPU par cœur**, `nvidia-smi`, `rocm-smi --json`. |
| Continue YAML | **OK** | Fusion + backup + détection bloc OAAS. |
| Continue JSON (ancien) | **OK** | Si seul `config.json` ou YAML absent ; fusion + backup. |
| Continue **workspace** | **OK** | `GET /oaas/ide/continue-status?workspace=…` + POST `workspace` ; UI : champ **projet** → `<projet>/.continue/…`. |
| ROCm parsing | **Partiel** | Dépend des clés JSON de ta version ROCm ; ajuster si besoin. |
| RAG / embeddings | **Absent** | Prévu. |
| **MemePalace** (palais de la mémoire) | **Absent** | Idée notée : lier *méthode des lieux* à organisation doc / RAG / UX ; cadrage dans [BACKLOG.md](BACKLOG.md) — aucun module OAAS aujourd’hui. |
| Service systemd | **Partiel** | Exemple `scripts/oaas.service.example` (user unit) + README ; pas d’unité imposée par le dépôt. |

---

## Tests réels à faire (checklist)

- [ ] Machine **NVIDIA** : vérifier que la section GPU affiche nom + VRAM + %.
- [ ] Machine **AMD ROCm** : `rocm-smi --json` manuel puis recharger `/oaas/` — valider champs (ajuster parse si clés différentes).
- [ ] **Continue** : uniquement `config.json` → bouton fusion → fichier mis à jour + backup.
- [ ] **Continue** : `config.yaml` + `config.json` → écriture dans YAML (priorité amont).
- [ ] `oaas status` **sans** serveur puis **avec** `make serve` : TCP + HTTP UI passent à OK.
- [ ] `make status-ci` dans le dépôt (CI local) après modifications Rust.
- [ ] `make install-user` puis `oaas status` depuis `$HOME` (PATH).
- [x] `cargo test` : helpers JSON ROCm (`json_as_u32` avec chaînes « N % »), `/proc`, `classify_exe`.

---

## Historique des décisions utiles

- **API Continue** = URL du **proxy OAAS** (`…/v1`), pas le port interne llama seul → LLMLingua reste dans la boucle.
- **CPU par cœur** : deux lectures `/proc/stat` espacées de ~200 ms → pourcentage approximatif (suffisant pour un tableau de bord ; pas un profiler).
- **Rôles des fichiers** : **STATUS** = vision + état ; **TODOS** = cases à cocher ; **BACKLOG** = détail, investigations, historique de livraison ; **README** = installation et commandes.

---

*Dernière mise à jour : entrée **MemePalace** (palais de la mémoire) dans vision + tableau ; détail [BACKLOG.md](BACKLOG.md).*
