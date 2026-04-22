# OAAS — cibles pratiques (Linux ; macOS/Windows : utiliser cargo directement ou cross-compile)
#
# Installation système (souvent sudo) :
#   make release install PREFIX=/usr/local
#
# Installation utilisateur (sans sudo) :
#   make release install-user
#
# Variables :
#   PREFIX   répertoire d’installation (défaut: /usr/local)
#   DESTDIR  préfixe pour paquets (.deb, etc.), vide par défaut
#   CARGO    ex. cargo ou mold -run cargo

PREFIX ?= /usr/local
DESTDIR ?=
CARGO ?= cargo

BINDIR := $(DESTDIR)$(PREFIX)/bin
DATADIR := $(DESTDIR)$(PREFIX)/share/oaas
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
UNAME_S := $(shell uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]' || echo unknown)
UNAME_M := $(shell uname -m 2>/dev/null || echo unknown)
DISTNAME := oaas-$(VERSION)-$(UNAME_M)-$(UNAME_S)

.PHONY: all build release run serve doctor init-config config llmlingua-venv clean install install-user uninstall dist fmt clippy check help

help:
	@echo "Cibles principales :"
	@echo "  make / make build     — compilation debug"
	@echo "  make release          — binaire optimisé dans target/release/oaas"
	@echo "  make run              — cargo run -- serve (dev, sans bannière URL)"
	@echo "  make serve            — même chose + rappel des URLs /oaas/ avant le démarrage"
	@echo "  make doctor           — vérifie llama-server, config, modèle GGUF"
	@echo "  make init-config      — crée ~/.config/oaas/config.yaml depuis l’exemple"
	@echo "  make config           — alias de init-config"
	@echo "  make llmlingua-venv   — crée .venv-llmlingua et installe llmlingua (PyTorch à ajouter)"
	@echo "  make install          — installe le binaire release (PREFIX=$(PREFIX))"
	@echo "  make install-user     — installe dans ~/.local/bin + exemple dans ~/.local/share/oaas"
	@echo "  make uninstall        — retire l’installation PREFIX (voir help)"
	@echo "  make dist             — archive target/$(DISTNAME).tar.gz (après release)"
	@echo "  make fmt clippy check — qualité de code"
	@echo ""
	@echo "Cross-compilation (exemple) :"
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
	@echo "Démarrage OAAS (Ctrl+C pour arrêter)."
	@echo "  UI tableau de bord : http://127.0.0.1:11435/oaas/  (adapte le port à server.bind dans ~/.config/oaas/config.yaml)"
	@echo "  JSON statut profils  : même origine → /oaas/status.json"
	$(CARGO) run -- serve

doctor:
	$(CARGO) run -- doctor

init-config config:
	$(CARGO) run -- init-config

llmlingua-venv:
	python3 -m venv .venv-llmlingua
	./.venv-llmlingua/bin/pip install -U pip
	./.venv-llmlingua/bin/pip install -r scripts/requirements-llmlingua.txt
	@echo ""
	@echo "Installe PyTorch (CPU ou CUDA) selon https://pytorch.org puis vérifie : .venv-llmlingua/bin/python -c \"import llmlingua\""
	@echo "Exemple command (YAML) : [\"$(CURDIR)/.venv-llmlingua/bin/python\", \"-u\", \"$(CURDIR)/scripts/oaas_llmlingua_worker.py\"]"

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
	echo "Installé : $$h/.local/bin/oaas"; \
	echo "Ajoute $$h/.local/bin au PATH si ce n’est pas déjà fait."

uninstall:
	rm -f "$(BINDIR)/oaas"
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
	@tar -C target -czf "target/$(DISTNAME).tar.gz" "$(DISTNAME)"
	@echo "Archive : target/$(DISTNAME).tar.gz"
