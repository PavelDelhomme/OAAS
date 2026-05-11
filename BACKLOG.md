# OAAS — BACKLOG

Fichier **priorisé** : tout ce qu’il reste à faire en profondeur, les **problèmes à investiguer**, et un **historique récent** des livraisons.  
Pour la **vision** et le **tableau d’état des fonctionnalités**, voir **[STATUS.md](STATUS.md)**.  
Pour une **checklist courte** au quotidien, voir **[TODOS.md](TODOS.md)**.

---

## Problèmes / investigations

### Ollama qui se lance tout seul (`ollama serve`) et surcharge la machine

**Constat :** OAAS **ne lance pas** Ollama. Si `ollama` ou `ollama serve` revient sans que tu le demandes, la cause est en **dehors** du dépôt OAAS (service, session, autre outil).

**Causes fréquentes (Linux) :**

| Source | Vérification rapide |
|--------|---------------------|
| **Service systemd utilisateur** (souvent après `curl install ollama`) | `systemctl --user status ollama` puis `systemctl --user is-enabled ollama` |
| **Service systemd système** | `systemctl status ollama` (sans `--user`) |
| **Conteneur / Flatpak / Snap** | `docker ps`, `flatpak list`, `snap list` (selon ce que tu utilises) |
| **Autostart bureau** | `~/.config/autostart/*.desktop` contenant « ollama » |
| **Extension IDE** (Continue, autre) | certaines configs ou docs parlent encore d’Ollama ; l’extension ne remplace pas systemd mais peut inciter à le laisser tourner |

**Pour arrêter sans désinstaller :**

- Depuis le dépôt OAAS : `make stop-ollama` ou `make stop-all` (voir Makefile / README).
- Désactiver au démarrage (utilisateur) : `systemctl --user disable ollama` puis `systemctl --user stop ollama`.
- Service système : `sudo systemctl disable ollama` puis `sudo systemctl stop ollama` (selon ton install).

**Piste « à creuser » :** noter la sortie de `systemctl --user is-enabled ollama` et `journalctl --user -u ollama -n 30` après un boot où ça s’est relancé tout seul — ça permet d’ajuster cette section avec précision (version paquet, distro).

---

## À faire (fonctionnalités & technique)

### Court terme

1. Ajuster parse **ROCm** selon retours terrain (exemples JSON `rocm-smi` 5.x / 6.x).
2. ~~Option **workspace** `.continue/config.yaml`~~ → **Fait** : query `workspace`, POST, UI champ projet.
3. Tests **E2E** ou fichiers temporaires si besoin (complément aux tests unitaires `cargo test` déjà en place).

### Moyen terme

4. **RAG** : embeddings + index sur `XDG_DATA_HOME/oaas/docs` + route ou worker.
5. ~~**systemd** : unité exemple~~ → **Fait** : `scripts/oaas.service.example` + README ; affiner variables / chemins selon retours.
6. ~~Reprise~~ pour **gros** `models pull` : **fait** (fichier `.part` + `Range` HTTP) ; miroirs URL encore ouverts si besoin.

### Plus tard

7. Client Tauri ou GTK **optionnel** (même JSON qu’aujourd’hui).
8. Métriques **Intel GPU** / **Mesa** si besoin.

---

## MemePalace — technique du palais de la mémoire (*method of loci*)

**Nom projet :** *MemePalace* (jeu de mots possible avec *memory palace* / *meme* selon ta direction produit).

**Référence cognitive :** la **méthode des lieux** consiste à ancrer des informations dans un parcours spatial imaginaire (ou réel) pour faciliter mémorisation et rappel — souvent utilisée pour l’apprentissage dense.

**Lien possible avec OAAS (à cadrer) :**

- Structurer le **cache doc** ou les **réponses RAG** selon des « pièces » / parcours plutôt qu’une liste plate.
- **Prompts système** ou gabarits Continue qui guident l’utilisateur à placer des concepts sur un chemin mental.
- **UI `/oaas/`** : visualisation ou assistant pas à pas (hors scope technique immédiat).

**État actuel :** **aucune implémentation** dans le dépôt ; uniquement **suivi documentaire** (STATUS, ce fichier, TODOS). Prochaine étape : mini-spec (objectif utilisateur, hors / dans le LLM, dépendance RAG).

---

## Récemment fait (mémo courte)

| Période | Livré |
|---------|--------|
| Récent | Continue **workspace** (`?workspace=`, POST, UI), `scripts/oaas.service.example`, `make stop` / `doctor --fix` / `models-*`, tests `cargo test`, doc. |

*(Détail produit « état par domaine » : toujours dans STATUS.md.)*
