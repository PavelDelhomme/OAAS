# OAAS

Orchestrateur **local** pour faire tourner des LLM avec **llama.cpp** (`llama-server`), une API **compatible OpenAI** (`/v1/chat/completions`, etc.) pour **Continue**, Emacs, Neovim, etc.

**Ce projet ne dépend pas d’Ollama.** Tu peux arrêter Ollama et ne garder que OAAS + `llama-server` : dans Continue, mets la base URL sur `http://127.0.0.1:11435/v1` (ou le `server.bind` de ton YAML).

## Fonctionnalités

- Lancement et arrêt propres de **llama-server** (profils YAML).
- **Proxy** HTTP vers l’API OpenAI-like de llama-server.
- **Compression automatique des prompts** avec [LLMLingua](https://github.com/microsoft/LLMLingua) (worker Python) pour les `POST /v1/chat/completions`, si `prompt_compression.enabled` est `true`.
- Commandes : `serve`, `doctor`, `init-config`.
- **Makefile** : build, install, venv LLMLingua, etc.

## Prérequis

- **Rust** (edition 2021), **make**, **llama-server** dans le `PATH` ou `runtime.llama_server_binary` dans le YAML.
- Un fichier modèle **`.gguf`** (poids quantifiés pour llama.cpp).
- Pour LLMLingua : **Python 3** + venv (voir plus bas).

## Démarrage rapide

```bash
make init-config
# Édite ~/.config/oaas/config.yaml : profiles.default.model = chemin absolu vers ton .gguf
make llmlingua-venv
# Installe PyTorch (CPU ou CUDA) dans le venv : https://pytorch.org
# Mets à jour prompt_compression.command dans le YAML (chemins absolus vers python du venv + worker)
make doctor
make serve
```

Sans LLMLingua pour tester vite : dans le YAML, `prompt_compression.enabled: false`.

## GitHub en SSH

1. Crée un dépôt vide sur GitHub (sans README s’il est déjà ici).
2. Depuis ce dossier :

```bash
git init
git add -A
git commit -m "Initial import OAAS"
git branch -M main
git remote add origin git@github.com:TON_USER/TON_REPO.git
ssh -T git@github.com   # doit dire « Hi TON_USER »
git push -u origin main
```

Remplace `TON_USER/TON_REPO` par ton compte et le nom du dépôt. Si `git push` échoue, configure une [clé SSH](https://docs.github.com/en/authentication/connecting-to-github-with-ssh) ou utilise `ssh-add` pour charger la clé.

## Continue

- **API base (OpenAI-compatible)** : `http://127.0.0.1:11435/v1` (adapter au `server.bind`).
- **Modèle** : celui exposé par `llama-server` (souvent le nom du GGUF ou l’id renvoyé par `GET /v1/models`).
- Clé API : une valeur factice suffit en local si l’UI en demande une.

## LLMLingua

Le démon Rust lance un **processus Python** qui charge LLMLingua une fois, puis compresse chaque requête de chat via IPC binaire. Paramètres principaux dans le YAML : `rate`, `target_token`, `model_name`, `use_llmlingua2`, `device_map`, `strict` (si `false`, en cas d’erreur la requête part **sans** compression).

Référence amont : [microsoft/LLMLingua](https://github.com/microsoft/LLMLingua).

## Licence

Voir le fichier `LICENSE` (MIT).
