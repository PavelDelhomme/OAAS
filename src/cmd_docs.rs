use clap::Subcommand;
use tokio::process::Command;
use tracing::info;

use crate::docs_catalog::{doc_pack_dest, find_doc_pack, load_docs_catalog};
use crate::download::download_url_to_file;
use crate::error::OaasError;
use crate::proxy::build_client;

const DEFAULT_DEVDOCS_IMAGE: &str = "ghcr.io/freecodecamp/devdocs:latest";

#[derive(Subcommand)]
pub enum DocsCommand {
    /// Liste les paquets de documentation disponibles.
    List,
    /// Télécharge ou clone un paquet dans le cache OAAS.
    Pull {
        id: String,
        #[arg(long)]
        force: bool,
        /// Pour `devdocs-stack` : lance aussi le conteneur Docker (nom `oaas-devdocs`).
        #[arg(long)]
        docker: bool,
    },
    /// Répertoire de cache pour un id.
    Path { id: String },
    /// Aide DevDocs (Docker, devdocs.io, clone Ruby) — même approche que https://github.com/freeCodeCamp/devdocs
    Devdocs {
        #[arg(
            long,
            help = "Lance « docker run » (conteneur oaas-devdocs, port 9292)"
        )]
        docker: bool,
    },
}

pub async fn run_docs(cmd: DocsCommand) -> Result<(), OaasError> {
    let catalog = load_docs_catalog()?;
    match cmd {
        DocsCommand::List => {
            println!("Paquets documentation (v{}) :\n", catalog.version);
            for p in &catalog.packs {
                println!("  {:<18} [{}] {}", p.id, p.method, p.title);
                if !p.note_fr.is_empty() {
                    println!("                     {}", p.note_fr);
                }
            }
            println!("\nDevDocs (référence) : oaas docs devdocs");
            println!("Compléments cache   : oaas docs pull <id> [--docker]");
        }
        DocsCommand::Pull { id, force, docker } => {
            let p = find_doc_pack(&catalog, &id)
                .ok_or_else(|| OaasError::Config(format!("doc « {id} » inconnue du catalogue")))?;
            let dest = doc_pack_dest(&id);
            match p.method.as_str() {
                "devdocs" => {
                    install_devdocs_notes(&dest, p, force).await?;
                    if docker {
                        try_devdocs_docker(p, force).await?;
                    }
                }
                "git" => {
                    if dest.exists() && !force {
                        info!(path = %dest.display(), "dossier déjà présent (--force pour recloner)");
                    } else {
                        if dest.exists() {
                            tokio::fs::remove_dir_all(&dest).await.map_err(|e| {
                                OaasError::Backend(format!("suppression {}: {e}", dest.display()))
                            })?;
                        }
                        let branch = if p.branch.is_empty() {
                            "main"
                        } else {
                            p.branch.as_str()
                        };
                        let st = Command::new("git")
                            .args([
                                "clone",
                                "--depth",
                                "1",
                                "-b",
                                branch,
                                &p.url,
                                dest.to_str().ok_or_else(|| {
                                    OaasError::Config("chemin doc non UTF-8".into())
                                })?,
                            ])
                            .status()
                            .await
                            .map_err(|e| {
                                OaasError::Backend(format!(
                                    "git clone — installe git ou vérifie le réseau: {e}"
                                ))
                            })?;
                        if !st.success() {
                            return Err(OaasError::Backend(
                                "git clone a échoué (voir la sortie ci-dessus)".into(),
                            ));
                        }
                    }
                }
                "fetch" => {
                    if p.url.is_empty() {
                        return Err(OaasError::Config(
                            "URL vide pour ce paquet (vérifie docs_catalog.yaml)".into(),
                        ));
                    }
                    tokio::fs::create_dir_all(&dest).await.map_err(|e| {
                        OaasError::Backend(format!("mkdir {}: {e}", dest.display()))
                    })?;
                    let file = dest.join("index.html");
                    if file.exists() && !force {
                        info!(path = %file.display(), "fichier déjà présent (--force pour retélécharger)");
                    } else {
                        let client = build_client()?;
                        download_url_to_file(&client, &p.url, &file).await?;
                    }
                }
                other => {
                    return Err(OaasError::Config(format!(
                        "méthode « {other} » non supportée (git|fetch|devdocs)"
                    )));
                }
            }
            println!("{}", dest.display());
        }
        DocsCommand::Path { id } => {
            if find_doc_pack(&catalog, &id).is_none() {
                return Err(OaasError::Config(format!("doc « {id} » inconnue")));
            }
            println!("{}", doc_pack_dest(&id).display());
        }
        DocsCommand::Devdocs { docker } => {
            print_devdocs_intro();
            if docker {
                let fake = crate::docs_catalog::DocPack {
                    id: "devdocs-cli".into(),
                    title: String::new(),
                    method: "devdocs".into(),
                    url: String::new(),
                    branch: String::new(),
                    note_fr: String::new(),
                    docker_image: DEFAULT_DEVDOCS_IMAGE.to_string(),
                    docker_port: Some(9292),
                };
                try_devdocs_docker(&fake, false).await?;
            }
        }
    }
    Ok(())
}

fn print_devdocs_intro() {
    println!(
        "DevDocs — https://github.com/freeCodeCamp/devdocs\n\
         \n\
         Même idée que devdocs.io : plusieurs documentations officielles, recherche rapide, offline.\n\
         \n\
         • Hébergé (zéro install) : https://devdocs.io\n\
         • Docker local (recommandé par upstream) :\n\
           docker run -d --name oaas-devdocs -p 9292:9292 {}\n\
           → http://localhost:9292\n\
         • Installation manuelle (Ruby + thor) : voir le README du dépôt (bundle exec thor docs:download …).\n\
         \n\
         OAAS : « oaas docs pull devdocs-stack » écrit un rappel dans le cache ; « --docker » tente le run.\n\
         Image par défaut : {}\n",
        DEFAULT_DEVDOCS_IMAGE, DEFAULT_DEVDOCS_IMAGE
    );
}

async fn install_devdocs_notes(
    dest: &std::path::Path,
    p: &crate::docs_catalog::DocPack,
    force: bool,
) -> Result<(), OaasError> {
    tokio::fs::create_dir_all(dest)
        .await
        .map_err(|e| OaasError::Backend(format!("mkdir {}: {e}", dest.display())))?;
    let f = dest.join("OAAS_DEVDOCS.txt");
    if f.exists() && !force {
        info!(path = %f.display(), "notes DevDocs déjà présentes (--force pour réécrire)");
        return Ok(());
    }
    let image = if p.docker_image.is_empty() {
        DEFAULT_DEVDOCS_IMAGE
    } else {
        p.docker_image.as_str()
    };
    let port = p.docker_port.unwrap_or(9292);
    let body = format!(
        "DevDocs (freeCodeCamp) — https://github.com/freeCodeCamp/devdocs\n\n\
         OAAS n’embarque pas le scraper Ruby : on s’aligne sur l’outil amont.\n\n\
         Docker :\n  docker run -d --name oaas-devdocs -p {port}:{port} {image}\n  → http://localhost:{port}\n\n\
         Mise à jour des docs dans le conteneur : suivre la doc DevDocs (thor docs:download --installed en install manuelle).\n\n\
         devdocs.io (hébergé) : https://devdocs.io\n",
    );
    tokio::fs::write(&f, body)
        .await
        .map_err(|e| OaasError::Backend(format!("écriture {}: {e}", f.display())))?;
    Ok(())
}

async fn try_devdocs_docker(
    p: &crate::docs_catalog::DocPack,
    force: bool,
) -> Result<(), OaasError> {
    if which::which("docker").is_err() {
        return Err(OaasError::Backend(
            "docker introuvable dans le PATH — installe Docker ou lance les commandes du fichier OAAS_DEVDOCS.txt à la main.".into(),
        ));
    }
    let image = if p.docker_image.is_empty() {
        DEFAULT_DEVDOCS_IMAGE
    } else {
        p.docker_image.as_str()
    };
    let port = p.docker_port.unwrap_or(9292);
    let name = "oaas-devdocs";

    if force {
        let _ = Command::new("docker")
            .args(["rm", "-f", name])
            .status()
            .await;
    } else {
        let st = Command::new("docker")
            .args(["container", "inspect", name])
            .status()
            .await
            .map_err(|e| OaasError::Backend(format!("docker inspect: {e}")))?;
        if st.success() {
            info!(
                container = name,
                "conteneur déjà présent — pas de nouveau run (utilise --force sur pull)"
            );
            return Ok(());
        }
    }

    let st = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            name,
            "-p",
            &format!("{port}:{port}"),
            image,
        ])
        .status()
        .await
        .map_err(|e| OaasError::Backend(format!("docker run: {e}")))?;
    if !st.success() {
        return Err(OaasError::Backend(
            "docker run a échoué — vérifie les ports, les permissions, ou lance la commande affichée dans OAAS_DEVDOCS.txt".into(),
        ));
    }
    info!(
        %image,
        port,
        url = %format!("http://127.0.0.1:{port}/"),
        "conteneur DevDocs démarré"
    );
    Ok(())
}
