# OAAS

**OAAS** signifie **Own AI Agents System** : c’est le nom du composant **orchestrateur** dans l’écosystème **OwnAiAgentsSystem** (ton dossier parent). En pratique, c’est un serveur local qui enchaîne **llama.cpp**, proxy HTTP et UI — pas un « cloud » ni Ollama.

Orchestrateur **local** pour faire tourner des LLM avec **llama.cpp** (`llama-server`), une API **compatible OpenAI** (`/v1/chat/completions`, etc.) pour **Continue**, Emacs, Neovim, etc.

**Ce projet ne dépend pas d’Ollama.** Tu peux arrêter Ollama et ne garder que OAAS + `llama-server` : dans Continue, mets la base URL sur `http://127.0.0.1:11435/v1` (ou le `server.bind` de ton YAML).

**Suivi projet** : **[STATUS.md](STATUS.md)** (vision + état des features) · **[TODOS.md](TODOS.md)** (checklist) · **[BACKLOG.md](BACKLOG.md)** (backlog détaillé, ex. Ollama qui redémarre seul). Après `make install-user`, des copies peuvent être sous `~/.local/share/oaas/` pour STATUS (selon install).

---

## Que faire, dans quel ordre ?

| Étape | Commande | Rôle |
|--------|-----------|------|
| 1. Config | `make init-config` | Crée `~/.config/oaas/config.yaml` depuis l’exemple. |
| 2. Modèle | `make models-recommend` puis `make models-pull ID=<id>` (depuis le dépôt) ; ou `oaas models …` après `make install-user` | Télécharge un GGUF et met à jour le YAML (`--patch-config`). |
| 3. Vérif | `make doctor` | Vérifie `llama-server`, le YAML et le fichier `.gguf`. |
| (optionnel) | `make doctor-fix` ou `cargo run -- doctor --fix` | Si LLMLingua est activé mais cassé (Python), met `prompt_compression.enabled: false` dans le YAML. |
| 4. Lancer | `make serve` | Démarre **llama-server** + **proxy** + **UI** (`/oaas/`). **Arrêt** : `Ctrl+C` (puis relance avec `make serve`). |
| (à tout moment) | **`make status`** ou `oaas status` | Résumé : config présente, `.gguf`, port OAAS, UI HTTP, bloc Continue, type de binaire (`debug` / `release` / install). |
| (CI local) | **`make status-ci`** | Comme `status` + `cargo fmt --check`, `clippy`, `build` dans le dépôt (plus lent). |

**Développement du dépôt** (logs + hot reload mental : tu relances après edit Rust) :

- `make dev` — `RUST_LOG=warn,oaas=debug,tower_http=debug,…` + `cargo run -- serve`
- `make dev-trace` — `RUST_LOG=trace`
- `make run` / `make serve` — sans surcharge de logs (`serve` affiche les URLs)

**Après `make install-user`** : utilise le binaire `oaas` du `PATH` (`oaas serve`, `oaas doctor`, …). Le Makefile sert surtout dans le clone du dépôt.

**Interface web** : `http://127.0.0.1:11435/oaas/` (adapte au `server.bind`). JSON utiles :

- `/oaas/status.json` — profils, URL Continue, picks modèles.
- `/oaas/catalog.json` — catalogue GGUF.
- **`/oaas/system.json`** — Linux : charge, RAM, **CPU par cœur** (instantané ~200 ms), RSS OAAS + llama, **GPU NVIDIA** (`nvidia-smi`) et **AMD ROCm** (`rocm-smi --json` si disponible).
- **`/oaas/workstation.json`** — même logique que `oaas status` (profil = celui du serveur en cours).

---

## Fonctionnalités (rappel)

- Lancement et arrêt propres de **llama-server** (profils YAML).
- **Proxy** HTTP vers l’API OpenAI-like de llama-server.
- **Tableau de bord** **`/oaas/`** : profils, URL Continue, recommandations, catalogue, pont **Continue** (`/oaas/ide/*`), **métriques** (`/oaas/system.json`), synthèse poste (`/oaas/workstation.json`).
- **`oaas status`** / **`make status`** : état du poste sans lire toute la doc.
- **`oaas models pull`** : cache `XDG_DATA_HOME/oaas/models`.
- **`oaas docs pull`** : cache doc (`data/docs_catalog.yaml`).
- **LLMLingua** (optionnel, YAML).

### Instruct vs Coder (Qwen)

- **Instruct** : spec, doc longue, organisation.
- **Coder** : code, patches, arbre source.

Voir **`oaas models recommend`**.

---

## Makefile (résumé)

| Cible | Effet |
|--------|--------|
| `make` / `make help` | Aide (défaut). |
| `make build` | `cargo build` |
| `make release` | Binaire optimisé |
| `make serve` | URLs + `cargo run -- serve` |
| `make run` | `cargo run -- serve` sans bannière |
| `make dev` / `make dev-trace` | Serve + logs `RUST_LOG` |
| `make stop` | Tue `oaas` + `llama-server` si tu as perdu le terminal (sinon **Ctrl+C** dans le terminal où tourne `serve`). |
| `make stop-ollama` / `make stop-all` | Arrêt Ollama (systemd user ou indication) ; `stop-all` = OAAS + Ollama. |
| `make doctor` | Diagnostics |
| `make doctor-fix` / `make doctor-fix-issues` | `doctor --fix` : corrige LLMLingua incohérent dans le YAML |
| `make status` / `make status-ci` | `oaas status` [+ CI] |
| `make init-config` / `make config` | Création YAML utilisateur |
| `make models-recommend` / `make models-list` / `make models-pull ID=…` | Équivalent `cargo run -- models …` sans installer `oaas` dans le PATH |
| `make docs-list` | Liste le catalogue doc (`cargo run -- docs list`) |
| `make install-user` / `make uninstall-user` | `~/.local/bin` + `~/.local/share/oaas` |
| `make install` / `make uninstall` | `PREFIX` (ex. `/usr/local`) |
| `make dist` | Archive `target/oaas-*.tar.gz` |
| `make check` | `fmt` + `clippy` + `release` |

---

## Variables d’environnement

| Variable | Effet |
|-----------|--------|
| `OAAS_PROFILE` | Profil YAML utilisé par `oaas serve` / `doctor` (défaut : `default`). |
| `OAAS_MODELS_CATALOG` | Chemin absolu du catalogue modèles YAML. |
| `OAAS_DOCS_CATALOG` | Chemin absolu du catalogue doc YAML. |
| `RUST_LOG` | Niveau de logs Rust (`debug`, `trace`, filtres par module…). |

Après `make install`, le catalogue embarqué est lu depuis `../share/oaas/data/` relatif au binaire.

**Gros GGUF** : `oaas models pull` écrit d’abord un fichier `*.part` puis renomme ; si le téléchargement est interrompu, **relancer la même commande** reprend via l’en-tête HTTP `Range` (tant que le `.part` est intact).

---

## Prérequis

- **Rust**, **make**, **`llama-server`** (ex. paquets AUR `llama.cpp` sur Arch — [ArchWiki Llama.cpp](https://wiki.archlinux.org/title/Llama.cpp)).
- **`git`** pour `oaas docs pull` en mode `git`.

---

## Continue

- **API base** : `http://127.0.0.1:11435/v1` (selon `server.bind`).
- **Modèle** : exposé par llama-server (`GET /v1/models`).
- **Global** : si `~/.continue/config.yaml` existe, il est prioritaire sur `config.json` (comportement Continue amont). Sinon OAAS peut créer / fusionner dans **`config.json`** (format ancien).
- **Workspace** : si tu renseignes le champ **projet** sur `/oaas/` (répertoire sous `$HOME`), la fusion et le statut Continue visent **`<projet>/.continue/config.yaml`** (ou `.json`). API : `GET /oaas/ide/continue-status?workspace=~/chemin` et `POST /oaas/ide/apply-continue` avec `{ "workspace": "~/chemin" }` (en complément de `model` optionnel).
- **Sauvegarde** : avant chaque fusion, copie du fichier existant dans le même dossier avec suffixe `.oaas-backup.<timestamp_unix>`.
- **Config auto** : bloc **Continue + éditeur** sur `/oaas/` ou `POST /oaas/ide/apply-continue`.

## systemd (optionnel)

Après `make install-user`, voir **`scripts/oaas.service.example`** (copie aussi sous `~/.local/share/oaas/scripts/`). Copie vers `~/.config/systemd/user/oaas.service`, adapte `ExecStart` ou `Environment=OAAS_PROFILE=…` si besoin, puis :

`systemctl --user daemon-reload` · `systemctl --user enable --now oaas`

Instructions détaillées en tête du fichier `.example`.

## Observabilité

- **UI** : section « Système » (rafraîchissement périodique + bouton manuel).
- **CLI / JSON** : `oaas status --json` pour scripts ; `rocm-smi` absent → section AMD vide (normal sur machine sans ROCm).
- **Précision** : les pourcentages CPU cœur sont une **approximation** courte fenêtre ; pour du profiling fin utiliser `perf`, `bpftrace`, etc.

## LLMLingua

`config.example.yaml` et `make llmlingua-venv`. Référence : [microsoft/LLMLingua](https://github.com/microsoft/LLMLingua).

## Documentation (`oaas docs`)

```bash
oaas docs list
oaas docs pull rust-book
```

## GitHub en SSH

`git remote add origin git@github.com:USER/REPO.git` puis `git push -u origin main` après [clé SSH](https://docs.github.com/en/authentication/connecting-to-github-with-ssh).

## Licence

`LICENSE` (MIT).
