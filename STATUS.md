# OAAS — suivi projet (STATUS)

Document **vivant** : objectifs finaux, état réel des fonctionnalités, tests à faire, backlog ordonné. À mettre à jour quand une ligne change d’état.

---

## Vision cible (ce que je veux vraiment à la fin)

1. **Orchestrateur local** fiable : `llama-server` + proxy **OpenAI-compatible** + profils YAML, **sans Ollama**.
2. **Expérience développeur** : `make` / `oaas` clairs (`status`, `doctor`, `serve`, `dev`), install user ou système, archive `dist`.
3. **Interface** : tableau de bord `/oaas/` (catalogue, profils, Continue, métriques machine).
4. **Continue / IDE** : fusion **YAML ou JSON**, sauvegarde avant écriture, ouverture projet dans `code` / `cursor` / `codium`.
5. **Observabilité** : RAM / charge / **CPU par cœur** / **NVIDIA** + **AMD ROCm** (quand outils présents), RSS OAAS + llama.
6. **Suite** (pas encore livré) : index local + **RAG** sur cache doc, miroirs téléchargement, alignement versions outils (`go version`, etc.).

---

## État actuel (fonctionnel / partiel / absent)

| Domaine | État | Détail |
|--------|------|--------|
| Proxy `/v1/*` | **OK** | Chat, models, streaming ; LLMLingua optionnel. |
| Catalogue modèles + `pull` | **OK** | YAML + CLI + UI. |
| Docs `oaas docs` | **Partiel** | git/fetch/devdocs-notes ; pas de RAG. |
| UI `/oaas/` | **OK** | Statut, catalogue, IDE, système, navigation. |
| `oaas status` / `make status` | **OK** | Config, GGUF, TCP, HTTP `/oaas/`, Continue, binaire courant ; `--ci` = fmt+clippy+build. |
| `GET /oaas/workstation.json` | **OK** | Même synthèse pendant `serve` (profil actif du serveur). |
| Métriques `/oaas/system.json` | **OK (Linux)** | Charge, RAM, RSS, **CPU par cœur**, `nvidia-smi`, `rocm-smi --json`. |
| Continue YAML | **OK** | Fusion + backup + détection bloc OAAS. |
| Continue JSON (ancien) | **OK** | Si seul `config.json` ou YAML absent ; fusion + backup. |
| ROCm parsing | **Partiel** | Dépend des clés JSON de ta version ROCm ; ajuster si besoin. |
| RAG / embeddings | **Absent** | Prévu. |
| Service systemd | **Absent** | Redémarrage = Ctrl+C + `make serve` (documenté). |

---

## Tests réels à faire (checklist)

- [ ] Machine **NVIDIA** : vérifier que la section GPU affiche nom + VRAM + %.
- [ ] Machine **AMD ROCm** : `rocm-smi --json` manuel puis recharger `/oaas/` — valider champs (ajuster parse si clés différentes).
- [ ] **Continue** : uniquement `config.json` → bouton fusion → fichier mis à jour + backup.
- [ ] **Continue** : `config.yaml` + `config.json` → écriture dans YAML (priorité amont).
- [ ] `oaas status` **sans** serveur puis **avec** `make serve` : TCP + HTTP UI passent à OK.
- [ ] `make status-ci` dans le dépôt (CI local) après modifications Rust.
- [ ] `make install-user` puis `oaas status` depuis `$HOME` (PATH).

---

## Backlog ordonné (concret)

### Court terme

1. Ajuster parse **ROCm** selon retours terrain (exemples JSON `rocm-smi` 5.x / 6.x).
2. Option **workspace** `.continue/config.yaml` (en plus du global).
3. **Tests d’intégration** minimaux (parse `system_snapshot`, `project_status` avec fichiers temporaires).

### Moyen terme

4. **RAG** : embeddings + index sur `XDG_DATA_HOME/oaas/docs` + route ou worker.
5. **systemd** : unité `oaas.service` exemple dans `scripts/` + doc.
6. Reprise / miroirs pour **gros** `models pull`.

### Plus tard

7. Client Tauri ou GTK **optionnel** (même JSON qu’aujourd’hui).
8. Métriques **Intel GPU** / **Mesa** si besoin.

---

## Historique des décisions utiles

- **API Continue** = URL du **proxy OAAS** (`…/v1`), pas le port interne llama seul → LLMLingua reste dans la boucle.
- **CPU par cœur** : deux lectures `/proc/stat` espacées de ~200 ms → pourcentage approximatif (suffisant pour un tableau de bord ; pas un profiler).
- **STATUS.md** : suivi produit ; **README.md** : usage et références techniques.

---

*Dernière mise à jour : générée avec la livraison « status + CPU/ROCm + Continue JSON/backup + workstation.json ».*
