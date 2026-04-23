# OAAS — Makefile (Linux ; ailleurs : cargo directement)
#
# Variables : PREFIX=/usr/local  DESTDIR=  CARGO=cargo
#
# Aide : make help   (cible par défaut)

.DEFAULT_GOAL := help

PREFIX ?= /usr/local
DESTDIR ?=
CARGO ?= cargo

BINDIR := $(DESTDIR)$(PREFIX)/bin
DATADIR := $(DESTDIR)$(PREFIX)/share/oaas
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
UNAME_S := $(shell uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]' || echo unknown)
UNAME_M := $(shell uname -m 2>/dev/null || echo unknown)
DISTNAME := oaas-$(VERSION)-$(UNAME_M)-$(UNAME_S)

.PHONY: all build release run serve dev dev-trace stop stop-ollama stop-all doctor doctor-fix doctor-fix-issues init-config config llmlingua-venv models-recommend models-list models-info models-path models-pull docs-list clean install install-user uninstall uninstall-user dist fmt clippy check help status status-ci

help:
	@echo "═══════════════════════════════════════════════════════════════════"
	@echo " OAAS — aide Makefile"
	@echo "═══════════════════════════════════════════════════════════════════"
	@echo ""
	@echo "■ Première fois (dans le dépôt cloné)"
	@echo "  make init-config     → ~/.config/oaas/config.yaml depuis l’exemple"
	@echo "  make models-recommend → idem « oaas models recommend » (sans install)"
	@echo "  make models-pull ID=<id> → télécharge le GGUF et patch le YAML (--patch-config)"
	@echo "  (après install-user : oaas models … à la place de make models-*)"
	@echo "  make doctor          → vérifie llama-server, YAML, fichier .gguf"
	@echo "  make doctor-fix      → doctor --fix : désactive LLMLingua dans le YAML si import / script / commande cassés"
	@echo "  make doctor-fix-issues → alias de doctor-fix"
	@echo "  make status          → oaas status (config, GGUF, port, Continue, …)"
	@echo "  make status-ci       → idem + cargo fmt --check, clippy, build (lent)"
	@echo ""
	@echo "■ Lancer le serveur (proxy + UI web + llama-server)"
	@echo "  make serve           → rappelle les URLs /oaas/ puis cargo run -- serve"
	@echo "  make run             → pareil sans bannière (idéal scripts)"
	@echo "  make dev             → logs détaillés (RUST_LOG=…) + serve"
	@echo "  make dev-trace       → encore plus verbeux"
	@echo ""
	@echo "■ Arrêter (libère RAM / VRAM)"
	@echo "  Dans le terminal où tourne serve/dev/run : Ctrl+C  (méthode normale)"
	@echo "  make stop            → tue les processus « oaas » et « llama-server » (terminal perdu / bloqué)"
	@echo "  make stop-ollama     → arrêt Ollama : systemd --user, ou message si service système"
	@echo "  make stop-all        → make stop puis make stop-ollama"
	@echo "  Puis relance         → make serve"
	@echo ""
	@echo "■ Après install utilisateur (~/.local/bin/oaas)"
	@echo "  oaas serve           (ou make serve depuis le dépôt pour le dev)"
	@echo "  UI                    http://127.0.0.1:11435/oaas/  (selon server.bind)"
	@echo ""
	@echo "■ Packager / installer"
	@echo "  make release         → target/release/oaas"
	@echo "  make install-user    → ~/.local/bin + ~/.local/share/oaas/"
	@echo "  make uninstall-user  → retire cette install utilisateur"
	@echo "  make install         → PREFIX=$(PREFIX) (souvent sudo)"
	@echo "  make uninstall       → retire PREFIX (fichiers install/)"
	@echo "  make dist            → target/$(DISTNAME).tar.gz"
	@echo ""
	@echo "■ Qualité"
	@echo "  make build | fmt | clippy | check   (check = fmt + clippy + release)"
	@echo ""
	@echo "■ Variables d’environnement utiles (voir README)"
	@echo "  OAAS_PROFILE        profil YAML (défaut: default)"
	@echo "  OAAS_MODELS_CATALOG chemin catalogue modèles"
	@echo "  OAAS_DOCS_CATALOG   chemin catalogue doc"
	@echo "  RUST_LOG            ex. debug,oaas=trace,tower_http=debug"
	@echo ""
	@echo "■ Cross-compile (exemple)"
	@echo "  rustup target add x86_64-pc-windows-gnu"
	@echo "  $(CARGO) build --release --target x86_64-pc-windows-gnu"

all: build

build:
	$(CARGO) build

release:
	$(CARGO) build --release

run:
	$(CARGO) run -- serve

serve:
	@echo "Démarrage OAAS — arrêt : Ctrl+C dans ce terminal, ou depuis un autre : make stop"
	@echo "  UI : http://127.0.0.1:11435/oaas/  (adapte à server.bind dans ~/.config/oaas/config.yaml)"
	@echo "  JSON : /oaas/status.json  /oaas/system.json  /oaas/catalog.json"
	$(CARGO) run -- serve

dev:
	RUST_LOG=warn,oaas=debug,tower_http=debug,axum::rejection=trace $(CARGO) run -- serve

dev-trace:
	RUST_LOG=trace $(CARGO) run -- serve

# Arrêt « à la main » si tu n’as plus le terminal sous la main (sinon : Ctrl+C dans ce terminal).
# « cargo run » lance souvent un enfant nommé oaas ; on couvre aussi la ligne de commande complète.
stop:
	@echo "→ Arrêt OAAS + llama-server (ignore les erreurs si déjà arrêtés)."
	-pkill -f "oaas serve" 2>/dev/null || true
	-pkill -x oaas 2>/dev/null || true
	-pkill -x llama-server 2>/dev/null || true
	@echo "  Fait. Vérifie : pgrep -a oaas ; pgrep -a llama-server   (aucune ligne = OK)"

stop-ollama:
	@echo "→ Arrêt Ollama…"
	@if systemctl --user is-active --quiet ollama 2>/dev/null; then \
		systemctl --user stop ollama && echo "  Ollama (systemd --user) arrêté."; \
	elif systemctl is-active --quiet ollama 2>/dev/null; then \
		echo "  Ollama tourne en service système → lance : sudo systemctl stop ollama"; \
	else \
		echo "  Pas de service systemd « ollama » actif ; tentative pkill sur le binaire…"; \
		pkill -x ollama 2>/dev/null && echo "  Processus « ollama » terminé." || echo "  Rien à arrêter (ou Ollama sous autre nom / Flatpak)."; \
	fi

stop-all: stop stop-ollama

doctor:
	$(CARGO) run -- doctor

doctor-fix:
	$(CARGO) run -- doctor --fix

doctor-fix-issues: doctor-fix

status:
	$(CARGO) run -- status

status-ci:
	$(CARGO) run -- status --ci

init-config config:
	$(CARGO) run -- init-config

# Sous-commandes models / docs sans « oaas » dans le PATH (depuis la racine du dépôt).
models-recommend:
	$(CARGO) run -- models recommend

models-list:
	$(CARGO) run -- models list

models-info:
	@test -n "$(ID)" || (echo "Usage: make models-info ID=<id-du-catalogue>"; exit 1)
	$(CARGO) run -- models info "$(ID)"

models-path:
	@test -n "$(ID)" || (echo "Usage: make models-path ID=<id-du-catalogue>"; exit 1)
	$(CARGO) run -- models path "$(ID)"

models-pull:
	@test -n "$(ID)" || (echo "Usage: make models-pull ID=<id>   (optionnel: FORCE=1 pour --force)"; exit 1)
	$(CARGO) run -- models pull "$(ID)" --patch-config $(if $(filter 1 true yes,$(FORCE)),--force,)

docs-list:
	$(CARGO) run -- docs list

llmlingua-venv:
	python3 -m venv .venv-llmlingua
	./.venv-llmlingua/bin/pip install -U pip
	./.venv-llmlingua/bin/pip install -r scripts/requirements-llmlingua.txt
	@echo ""
	@echo "Installe PyTorch (CPU ou CUDA) selon https://pytorch.org puis :"
	@echo "  .venv-llmlingua/bin/python -c \"import llmlingua\""
	@echo "Exemple YAML : [\"$(CURDIR)/.venv-llmlingua/bin/python\", \"-u\", \"$(CURDIR)/scripts/oaas_llmlingua_worker.py\"]"

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy --all-targets -- -D warnings

check: fmt clippy release
	@echo "Vérifications OK."

clean:
	$(CARGO) clean

install: release
	install -d "$(BINDIR)" "$(DATADIR)" "$(DATADIR)/scripts" "$(DATADIR)/data" "$(DATADIR)/static"
	install -m755 target/release/oaas "$(BINDIR)/oaas"
	install -m644 config.example.yaml "$(DATADIR)/config.example.yaml"
	install -m755 scripts/oaas_llmlingua_worker.py "$(DATADIR)/scripts/oaas_llmlingua_worker.py"
	install -m644 scripts/requirements-llmlingua.txt "$(DATADIR)/scripts/requirements-llmlingua.txt"
	install -m644 data/models_catalog.yaml "$(DATADIR)/data/models_catalog.yaml"
	install -m644 data/docs_catalog.yaml "$(DATADIR)/data/docs_catalog.yaml"
	install -m644 static/oaas_ui.html "$(DATADIR)/static/oaas_ui.html"
	install -m644 STATUS.md "$(DATADIR)/STATUS.md"
	@echo "Installé : $(BINDIR)/oaas"
	@echo "Exemple YAML : $(DATADIR)/config.example.yaml"
	@echo "Si besoin : sudo make install PREFIX=$(PREFIX)"

install-user: release
	@set -e; h="$${HOME}"; \
	mkdir -p "$$h/.local/bin" "$$h/.local/share/oaas/scripts"; \
	install -m755 target/release/oaas "$$h/.local/bin/oaas"; \
	install -m644 config.example.yaml "$$h/.local/share/oaas/config.example.yaml"; \
	install -m755 scripts/oaas_llmlingua_worker.py "$$h/.local/share/oaas/scripts/oaas_llmlingua_worker.py"; \
	install -m644 scripts/requirements-llmlingua.txt "$$h/.local/share/oaas/scripts/requirements-llmlingua.txt"; \
	mkdir -p "$$h/.local/share/oaas/data" "$$h/.local/share/oaas/static"; \
	install -m644 data/models_catalog.yaml "$$h/.local/share/oaas/data/models_catalog.yaml"; \
	install -m644 data/docs_catalog.yaml "$$h/.local/share/oaas/data/docs_catalog.yaml"; \
	install -m644 static/oaas_ui.html "$$h/.local/share/oaas/static/oaas_ui.html"; \
	install -m644 STATUS.md "$$h/.local/share/oaas/STATUS.md"; \
	echo "Installé : $$h/.local/bin/oaas"; \
	echo "Ajoute $$h/.local/bin au PATH si besoin."

uninstall-user:
	@set -e; h="$${HOME}"; \
	rm -f "$$h/.local/bin/oaas"; \
	rm -f "$$h/.local/share/oaas/STATUS.md"; \
	rm -f "$$h/.local/share/oaas/config.example.yaml"; \
	rm -f "$$h/.local/share/oaas/scripts/oaas_llmlingua_worker.py" "$$h/.local/share/oaas/scripts/requirements-llmlingua.txt"; \
	rm -f "$$h/.local/share/oaas/data/models_catalog.yaml" "$$h/.local/share/oaas/data/docs_catalog.yaml"; \
	rm -f "$$h/.local/share/oaas/static/oaas_ui.html"; \
	rmdir "$$h/.local/share/oaas/scripts" "$$h/.local/share/oaas/data" "$$h/.local/share/oaas/static" 2>/dev/null || true; \
	rmdir "$$h/.local/share/oaas" 2>/dev/null || true; \
	echo "Désinstallé utilisateur (~/.local/bin/oaas + partage ~/.local/share/oaas)."

uninstall:
	rm -f "$(BINDIR)/oaas"
	rm -f "$(DATADIR)/STATUS.md"
	rm -f "$(DATADIR)/config.example.yaml"
	rm -f "$(DATADIR)/scripts/oaas_llmlingua_worker.py" "$(DATADIR)/scripts/requirements-llmlingua.txt"
	rm -f "$(DATADIR)/data/models_catalog.yaml" "$(DATADIR)/data/docs_catalog.yaml"
	rm -f "$(DATADIR)/static/oaas_ui.html"
	-rmdir "$(DATADIR)/scripts" "$(DATADIR)/data" "$(DATADIR)/static" 2>/dev/null || true
	-rmdir "$(DATADIR)" 2>/dev/null || true
	@echo "Désinstallé (PREFIX=$(PREFIX) DESTDIR=$(DESTDIR))."

dist: release
	@mkdir -p "target/$(DISTNAME)/scripts"
	@install -m755 target/release/oaas "target/$(DISTNAME)/oaas"
	@install -m644 config.example.yaml "target/$(DISTNAME)/config.example.yaml"
	@install -m755 scripts/oaas_llmlingua_worker.py "target/$(DISTNAME)/scripts/oaas_llmlingua_worker.py"
	@install -m644 scripts/requirements-llmlingua.txt "target/$(DISTNAME)/scripts/requirements-llmlingua.txt"
	@mkdir -p "target/$(DISTNAME)/data" "target/$(DISTNAME)/static"
	@install -m644 data/models_catalog.yaml "target/$(DISTNAME)/data/models_catalog.yaml"
	@install -m644 data/docs_catalog.yaml "target/$(DISTNAME)/data/docs_catalog.yaml"
	@install -m644 static/oaas_ui.html "target/$(DISTNAME)/static/oaas_ui.html"
	@install -m644 STATUS.md "target/$(DISTNAME)/STATUS.md"
	@tar -C target -czf "target/$(DISTNAME).tar.gz" "$(DISTNAME)"
	@echo "Archive : target/$(DISTNAME).tar.gz"
