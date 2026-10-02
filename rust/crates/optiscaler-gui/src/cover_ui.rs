//! Explicit, per-game artwork controls. All disk, dialog, and network work is off the UI thread.

use eframe::egui;
use opticore::cover_art::{CoverCache, GridGame, GridImage, SteamGridDb};
use opticore::i18n::Translator;
use opticore::model::Game;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

enum Action {
    Inspect,
    Local,
    Refresh,
    Reset,
    Search { token: String },
    Grids { token: String, id: u64 },
    Select { token: String, image: GridImage },
}

struct Request {
    id: u64,
    game: Game,
    cache: Arc<CoverCache>,
    action: Action,
    ctx: egui::Context,
}

enum Outcome {
    Inspected,
    Cancelled,
    Changed(Option<PathBuf>),
    Games(Vec<GridGame>),
    Images(Vec<GridImage>),
    Failed(String),
}

struct Reply {
    id: u64,
    key: String,
    source: Option<String>,
    outcome: Outcome,
}

fn run(request: Request) -> Reply {
    let Request {
        id,
        game,
        cache,
        action,
        ..
    } = request;
    let outcome = match action {
        Action::Inspect => Outcome::Inspected,
        Action::Local => match rfd::FileDialog::new()
            .add_filter("Image", &["jpg", "jpeg", "png", "webp"])
            .pick_file()
        {
            Some(path) => match cache.set_override(&game, &path) {
                Ok(path) => Outcome::Changed(Some(path)),
                Err(error) => Outcome::Failed(error),
            },
            None => Outcome::Cancelled,
        },
        Action::Refresh => Outcome::Changed(cache.refresh(&game)),
        Action::Reset => match cache.clear_override(&game) {
            Ok(()) => Outcome::Changed(None),
            Err(error) => Outcome::Failed(error),
        },
        Action::Search { token } => {
            match SteamGridDb::new(token).and_then(|db| db.search_game(&game)) {
                Ok(games) => Outcome::Games(games),
                Err(error) => Outcome::Failed(error),
            }
        }
        Action::Grids { token, id } => match SteamGridDb::new(token).and_then(|db| db.grids(id)) {
            Ok(images) => Outcome::Images(images),
            Err(error) => Outcome::Failed(error),
        },
        Action::Select { token, image } => match SteamGridDb::new(token) {
            Ok(db) => match db.select(&cache, &game, &image) {
                Ok(path) => Outcome::Changed(Some(path)),
                Err(_) => Outcome::Failed("Could not download or save the selected cover".into()),
            },
            Err(error) => Outcome::Failed(error),
        },
    };
    Reply {
        id,
        key: game.key.path_norm.clone(),
        source: cache.source(&game),
        outcome,
    }
}

pub struct CoverUi {
    game: Option<Game>,
    open: bool,
    token: String,
    consent: bool,
    source: Option<String>,
    needs_inspect: bool,
    search_attempted: bool,
    games: Vec<GridGame>,
    images: Vec<GridImage>,
    selected_game: Option<u64>,
    status: Option<String>,
    pending: Option<u64>,
    next_id: u64,
    last_action: Option<Instant>,
    requests: mpsc::Sender<Request>,
    replies: mpsc::Receiver<Reply>,
}

impl Default for CoverUi {
    fn default() -> Self {
        let (requests, incoming) = mpsc::channel::<Request>();
        let (outgoing, replies) = mpsc::channel::<Reply>();
        std::thread::spawn(move || {
            while let Ok(request) = incoming.recv() {
                let ctx = request.ctx.clone();
                let reply = run(request);
                if outgoing.send(reply).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            game: None,
            open: false,
            token: String::new(),
            consent: false,
            source: None,
            needs_inspect: false,
            search_attempted: false,
            games: Vec::new(),
            images: Vec::new(),
            selected_game: None,
            status: None,
            pending: None,
            next_id: 0,
            last_action: None,
            requests,
            replies,
        }
    }
}

impl CoverUi {
    pub fn open_for(&mut self, game: &Game) {
        if self.game.as_ref().map(|g| &g.key.path_norm) != Some(&game.key.path_norm) {
            self.source = None;
            self.needs_inspect = true;
            self.search_attempted = false;
            self.games.clear();
            self.images.clear();
            self.selected_game = None;
            self.status = None;
        }
        self.game = Some(game.clone());
        self.needs_inspect = true;
        self.open = true;
    }

    fn can_start(&self, consent_required: bool) -> bool {
        self.pending.is_none()
            && (!consent_required || (self.consent && !self.token.is_empty()))
            && self
                .last_action
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(1))
    }

    fn start(&mut self, ctx: &egui::Context, cache: Arc<CoverCache>, action: Action) {
        let Some(game) = self.game.clone() else {
            return;
        };
        let is_inspect = matches!(&action, Action::Inspect);
        let remote = matches!(
            &action,
            Action::Search { .. } | Action::Grids { .. } | Action::Select { .. }
        );
        if self.pending.is_some() || (!is_inspect && !self.can_start(remote)) {
            return;
        }
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        let request = Request {
            id,
            game,
            cache,
            action,
            ctx: ctx.clone(),
        };
        if self.requests.send(request).is_ok() {
            self.pending = Some(id);
            if is_inspect {
                self.needs_inspect = false;
            } else {
                self.last_action = Some(Instant::now());
            }
            self.status = None;
        } else {
            self.needs_inspect = false;
            self.status = Some("Artwork worker unavailable".into());
        }
    }

    fn poll(&mut self, tr: &Translator) -> Option<(String, Option<PathBuf>)> {
        let mut changed = None;
        loop {
            let reply = match self.replies.try_recv() {
                Ok(reply) => reply,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if self.pending.take().is_some() {
                        self.status = Some("Artwork worker unavailable".into());
                    }
                    break;
                }
            };
            if self.pending != Some(reply.id) {
                continue;
            }
            self.pending = None;
            let current = self
                .game
                .as_ref()
                .is_some_and(|g| g.key.path_norm == reply.key);
            if current {
                self.source = reply.source;
            }
            match reply.outcome {
                Outcome::Changed(path) => {
                    if current {
                        self.status = Some(tr.tr("library.success"));
                    }
                    changed = Some((reply.key, path));
                }
                Outcome::Games(games) if current => {
                    self.games = games;
                    self.images.clear();
                    self.selected_game = None;
                }
                Outcome::Images(images) if current => self.images = images,
                Outcome::Failed(error) if current => self.status = Some(error),
                _ => {}
            }
        }
        changed
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        tr: &Translator,
        cache: Arc<CoverCache>,
    ) -> Option<(String, Option<PathBuf>)> {
        let changed = self.poll(tr);
        if !self.open {
            return changed;
        }
        if let Some(remaining) = self
            .last_action
            .and_then(|at| Duration::from_secs(1).checked_sub(at.elapsed()))
        {
            ctx.request_repaint_after(remaining);
        }
        if self.needs_inspect && self.pending.is_none() {
            self.start(ctx, cache.clone(), Action::Inspect);
        }
        let Some(game) = self.game.as_ref() else {
            return changed;
        };
        let title = format!("{}: {}", tr.tr("library.cover_title"), game.name);
        let mut open = self.open;
        egui::Window::new(title)
            .id(egui::Id::new("cover_controls"))
            .open(&mut open)
            .resizable(true)
            .default_width(430.0)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{}: {}",
                    tr.tr("library.cover_source"),
                    self.source.as_deref().unwrap_or("-")
                ));
                if let Some(status) = &self.status {
                    ui.label(status);
                }
                if self.pending.is_some() {
                    ui.spinner();
                    ui.label(tr.tr("library.working"));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            self.can_start(false),
                            egui::Button::new(tr.tr("library.local_cover")),
                        )
                        .clicked()
                    {
                        self.start(ctx, cache.clone(), Action::Local);
                    }
                    if ui
                        .add_enabled(
                            self.can_start(false),
                            egui::Button::new(tr.tr("library.reset_cover")),
                        )
                        .clicked()
                    {
                        self.start(ctx, cache.clone(), Action::Reset);
                    }
                    if ui
                        .add_enabled(
                            self.can_start(false),
                            egui::Button::new(tr.tr("library.refresh_cover")),
                        )
                        .clicked()
                    {
                        self.start(ctx, cache.clone(), Action::Refresh);
                    }
                });
                ui.separator();
                ui.label(tr.tr("library.sgdb_privacy"));
                ui.checkbox(&mut self.consent, tr.tr("library.sgdb_consent"));
                ui.horizontal(|ui| {
                    ui.label(tr.tr("library.api_key"));
                    ui.add(egui::TextEdit::singleline(&mut self.token).password(true));
                });
                if ui
                    .add_enabled(
                        self.can_start(true),
                        egui::Button::new(tr.tr("library.search")),
                    )
                    .clicked()
                {
                    self.search_attempted = true;
                    self.games.clear();
                    self.images.clear();
                    self.selected_game = None;
                    self.start(
                        ctx,
                        cache.clone(),
                        Action::Search {
                            token: self.token.clone(),
                        },
                    );
                }
                if !self.games.is_empty() {
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("cover_game_candidates")
                        .max_height(170.0)
                        .show(ui, |ui| {
                            for candidate in self.games.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(&candidate.name);
                                    if ui
                                        .add_enabled(
                                            self.can_start(true),
                                            egui::Button::new(tr.tr("library.select_game")),
                                        )
                                        .clicked()
                                    {
                                        self.selected_game = Some(candidate.id);
                                        self.images.clear();
                                        self.start(
                                            ctx,
                                            cache.clone(),
                                            Action::Grids {
                                                token: self.token.clone(),
                                                id: candidate.id,
                                            },
                                        );
                                    }
                                });
                            }
                        });
                }
                if self.selected_game.is_some() {
                    ui.separator();
                    if self.images.is_empty() && self.pending.is_none() {
                        ui.label(tr.tr("library.no_results"));
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("cover_image_candidates")
                        .max_height(250.0)
                        .show(ui, |ui| {
                            for image in self.images.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("#{}", image.id));
                                    ui.hyperlink_to(tr.tr("library.open_preview"), &image.url);
                                    if ui
                                        .add_enabled(
                                            self.can_start(true),
                                            egui::Button::new(tr.tr("library.select_image")),
                                        )
                                        .clicked()
                                    {
                                        self.start(
                                            ctx,
                                            cache.clone(),
                                            Action::Select {
                                                token: self.token.clone(),
                                                image,
                                            },
                                        );
                                    }
                                });
                            }
                        });
                } else if self.search_attempted
                    && self.games.is_empty()
                    && self.pending.is_none()
                    && self.status.is_none()
                {
                    ui.label(tr.tr("library.no_results"));
                }
            });
        self.open = open;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_actions_require_consent_and_token() {
        let mut ui = CoverUi::default();
        assert!(!ui.can_start(true));
        ui.consent = true;
        assert!(!ui.can_start(true));
        ui.token = "key".into();
        assert!(ui.can_start(true));
        ui.consent = false;
        assert!(!ui.can_start(true));
        assert!(ui.can_start(false));
    }

    #[test]
    fn actions_are_serialized_and_rate_limited() {
        let mut ui = CoverUi {
            pending: Some(1),
            ..Default::default()
        };
        assert!(!ui.can_start(false));
        ui.pending = None;
        ui.last_action = Some(Instant::now());
        assert!(!ui.can_start(false));
        ui.last_action = Some(Instant::now() - Duration::from_secs(2));
        assert!(ui.can_start(false));
    }

    #[test]
    fn switching_game_clears_previous_candidates() {
        let mut ui = CoverUi::default();
        ui.open_for(&Game::new(
            "One",
            PathBuf::from("one"),
            opticore::model::Platform::Manual,
        ));
        ui.games.push(GridGame {
            id: 1,
            name: "One".into(),
        });
        ui.open_for(&Game::new(
            "Two",
            PathBuf::from("two"),
            opticore::model::Platform::Manual,
        ));
        assert!(ui.games.is_empty());
        assert_eq!(ui.game.as_ref().unwrap().name, "Two");
    }

    #[test]
    fn late_change_notifies_parent_without_changing_new_games_ui() {
        let mut ui = CoverUi::default();
        let (sender, replies) = mpsc::channel();
        ui.replies = replies;
        ui.open_for(&Game::new(
            "Two",
            PathBuf::from("two"),
            opticore::model::Platform::Manual,
        ));
        ui.pending = Some(7);
        let path = PathBuf::from("old-cover.jpg");
        sender
            .send(Reply {
                id: 7,
                key: "one".into(),
                source: Some("user override".into()),
                outcome: Outcome::Changed(Some(path.clone())),
            })
            .unwrap();

        assert_eq!(
            ui.poll(&Translator::default()),
            Some(("one".into(), Some(path)))
        );
        assert!(ui.source.is_none());
        assert!(ui.status.is_none());
        assert!(ui.poll(&Translator::default()).is_none());
    }

    #[test]
    fn late_search_cannot_populate_another_games_candidates() {
        let mut ui = CoverUi::default();
        let (sender, replies) = mpsc::channel();
        ui.replies = replies;
        ui.open_for(&Game::new(
            "Two",
            PathBuf::from("two"),
            opticore::model::Platform::Manual,
        ));
        ui.pending = Some(8);
        sender
            .send(Reply {
                id: 8,
                key: "one".into(),
                source: None,
                outcome: Outcome::Games(vec![GridGame {
                    id: 1,
                    name: "One".into(),
                }]),
            })
            .unwrap();
        assert!(ui.poll(&Translator::default()).is_none());
        assert!(ui.games.is_empty());
        assert!(ui.pending.is_none());
    }
}
