# OAAS

Orchestrateur **local** pour faire tourner des LLM avec **llama.cpp** (`llama-server`), une API **compatible OpenAI** (`/v1/chat/completions`, etc.) pour **Continue**, Emacs, Neovim, etc.

**Ce projet ne dépend pas d’Ollama.** Tu peux arrêter Ollama et ne garder que OAAS + `llama-server` : dans Continue, mets la base URL sur `http://127.0.0.1:11435/v1` (ou le `server.bind` de ton YAML).

## Fonctionnalités

- Lancement et arrêt propres de **llama-server** (profils YAML).
- **Proxy** HTTP vers l’API OpenAI-like de llama-server.
- **Catalogue de modèles GGUF** (Qwen 3.5, Coder vs Instruct, etc.) : CLI + **tableau de bord web** **`http://<bind>/oaas/`** (profils YAML, URL Continue, picks « recommend », filtre catalogue) + JSON **`/oaas/catalog.json`** et **`/oaas/status.json`**.
- **Pont Continue / éditeur** : depuis l’UI (ou les routes HTTP **`/oaas/ide/*`**), OAAS peut **fusionner** dans **`~/.continue/config.yaml`** un modèle nommé **« OAAS (local) »** pointant vers l’API **`…/v1`** du proxy (donc LLMLingua si activé), et lancer **`code` / `cursor` / `codium`** sur un dossier projet (**sous `$HOME` uniquement**). Continue doit être installé dans l’éditeur ; ce n’est pas un remplacement de Cursor, c’est la config + lancement pratiques.
- **`oaas models pull`** : téléchargement Hugging Face dans le cache `XDG_DATA_HOME/oaas/models` (réinstallable sur une autre machine : même compte utilisateur ou copie du dossier).
- **`oaas docs pull`** : premier socle pour mettre en cache de la doc (git / fetch) sous `XDG_DATA_HOME/oaas/docs/` (voir `data/docs_catalog.yaml`).
- **Compression des prompts** [LLMLingua](https://github.com/microsoft/LLMLingua) (optionnel, voir YAML).
- **Makefile** : build, install, venv LLMLingua, etc.

### Instruct vs Coder (Qwen)

- **Instruct** : polyvalent — spec, longues explications, doc, organisation de projet.
- **Coder** : orienté génération / relecture de code, patches, navigation de codebase.

Pour un usage « projet complet » : un **Instruct** raisonnable (ex. Qwen3.5 4B) sur machine modeste ; ajouter un **Coder** (ex. Qwen2.5 Coder 7B) si tu as assez de RAM/VRAM. Voir **`oaas models recommend`**.

## Prérequis

- **Rust**, **make**, **`llama-server`** (AUR `llama.cpp` ou `llama.cpp-vulkan` sur Arch — voir [ArchWiki Llama.cpp](https://wiki.archlinux.org/title/Llama.cpp)).
- Pour **`oaas docs pull`** avec `method: git` : **`git`** installé.

## Démarrage rapide

```bash
make init-config
oaas models recommend
oaas models pull qwen3.5-4b-q4km --patch-config
# (ou sans patch : copie le chemin affiché dans profiles.default.model)
make doctor
make serve
```

`make serve` affiche un rappel des URLs. Ouvre dans un navigateur **`http://127.0.0.1:11435/oaas/`** (adapte au `server.bind`) : catalogue filtrable, raccourcis équivalents à `oaas models recommend`, liste des **profils** et chemin **GGUF** actif, champ persistant (navigateur) pour noter ton **dossier projet** / Continue. L’API IDE reste **`…/v1`** (voir la page).

**Installation Linux** : `make release install-user` installe `~/.local/bin/oaas` + données sous `~/.local/share/oaas/` ; ajoute `~/.local/bin` au `PATH`. Pour une install système : `sudo make install PREFIX=/usr/local`. Archive portable : `make dist` → `target/oaas-*.tar.gz`.

Sans LLMLingua pour tester vite : dans le YAML, `prompt_compression.enabled: false`.

### Catalogue personnalisé

- Variable **`OAAS_MODELS_CATALOG`** : chemin vers un YAML de même forme que `data/models_catalog.yaml`.
- Après `make install`, le catalogue système est lu depuis **`../share/oaas/data/models_catalog.yaml`** (relatif au binaire).

### Documentation (`oaas docs`)

```bash
oaas docs list
oaas docs pull rust-book
oaas docs pull go-ref-spec
```

Les paquets sont décrits dans `data/docs_catalog.yaml`. C’est une **base** : la suite pourra lier version locale (`go version`, `git --version`) et des miroirs / mises à jour planifiées.

## GitHub en SSH

Voir les commandes dans l’historique du dépôt ; en résumé : `git remote add origin git@github.com:USER/REPO.git` puis `git push -u origin main` après [clé SSH](https://docs.github.com/en/authentication/connecting-to-github-with-ssh).

## Continue

- **API base** : `http://127.0.0.1:11435/v1` (selon `server.bind`).
- **Modèle** : celui exposé par `llama-server` (`GET /v1/models`).
- Clé API : valeur factice possible en local.

## LLMLingua

Voir `config.example.yaml` et `make llmlingua-venv`. Référence : [microsoft/LLMLingua](https://github.com/microsoft/LLMLingua).

## Licence

`LICENSE` (MIT).
