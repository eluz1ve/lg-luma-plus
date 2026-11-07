use super::*;
use crate::{model::BACKGROUNDS, renderer::Draw, wallpapers::Wallpaper};
use std::time::{Duration, Instant};

fn custom(index: u32) -> Wallpaper {
    Wallpaper {
        id: format!("custom-{index:032x}"),
        directory: PathBuf::from(format!("/preview/wallpapers/custom-{index:032x}")),
        has_original: true,
        frames: Vec::new(),
    }
}

#[test]
fn gallery_switches_sit_right_of_the_title_and_stay_fixed_in_every_language() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    for language in crate::i18n::Language::ALL {
        state.preferences.language = language;
        state.scroll = 0.;
        let before = controls(&state);
        let at = |target: Target| before.iter().find(|c| c.target == target).unwrap().rect;
        let cycle = at(Target::WallpaperCycle);
        let gradient = at(Target::WallpaperGradient);
        assert_eq!(cycle.x + cycle.w, 1840.);
        assert_eq!(gradient.x + gradient.w + 16., cycle.x);
        assert!(gradient.x >= 640., "{}: {}", language.code(), gradient.x);
        assert_eq!((cycle.y, cycle.h), (148., 56.));
        assert_eq!(cycle.y, gradient.y);
        state.scroll = 500.;
        let after = controls(&state);
        for target in [Target::WallpaperCycle, Target::WallpaperGradient] {
            let old = before.iter().find(|c| c.target == target).unwrap();
            let new = after.iter().find(|c| c.target == target).unwrap();
            assert_eq!(
                (old.rect.x, old.rect.y, old.rect.w),
                (new.rect.x, new.rect.y, new.rect.w)
            );
            assert!(!new.scrollable);
        }
    }
}

#[test]
fn cycling_button_toggles_on_off_without_opening_a_page_or_moving_controls() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::WallpaperGradient;
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperCycle);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperGradient);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(1));
    state.focus = Target::WallpaperGradient;
    state.direction(Direction::Right, &controls(&state));
    assert!(!state.preferences.wallpaper_cycle);
    state.scroll = 300.;
    for language in crate::i18n::Language::ALL {
        state.preferences.language = language;
        let before = controls(&state);
        assert!(scene(&state).draw.iter().any(|draw| matches!(draw, Draw::TintedImage(path, ..) if path.ends_with("ui-icons/refresh-off.lvec"))));
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert!(state.preferences.wallpaper_cycle);
        assert!(state.preferences.wallpaper_changed > 0);
        assert!(state.subpage.is_none());
        assert!(state.overlay.is_none());
        assert_eq!(state.focus, Target::WallpaperCycle);
        assert_eq!(state.scroll, 300.);
        let enabled = scene(&state);
        assert!(enabled.draw.iter().any(|draw| matches!(draw, Draw::TintedImage(path, ..) if path.ends_with("ui-icons/refresh.lvec"))));
        assert!(!enabled.draw.iter().any(|draw| matches!(draw, Draw::TintedImage(path, ..) if path.ends_with("ui-icons/refresh-off.lvec"))));
        let label = language.t("wallpaperCycle");
        assert!(
            enabled
                .draw
                .iter()
                .any(|draw| matches!(draw, Draw::Text(value, ..) if value == label))
        );
        let button = enabled
            .controls
            .iter()
            .find(|c| c.target == Target::WallpaperCycle)
            .unwrap()
            .rect;
        let title = enabled
            .draw
            .iter()
            .find_map(|draw| match draw {
                Draw::Text(value, rect, ..) if value == label => Some(rect),
                _ => None,
            })
            .unwrap();
        let (value, style) = enabled
            .draw
            .iter()
            .find_map(|draw| match draw {
                Draw::Text(label, rect, style, ..)
                    if label == language.t("stateOn") && rect.x > button.x =>
                {
                    Some((rect, style))
                }
                _ => None,
            })
            .unwrap();
        assert!(style.align_right);
        assert!((value.x - title.x - title.w - 12.).abs() < 0.001);
        assert!((button.x + button.w - value.x - value.w - 24.).abs() < 0.001);
        for (old, new) in before.iter().zip(&enabled.controls) {
            assert_eq!(old.target, new.target);
            assert_eq!(
                (old.rect.x, old.rect.y, old.rect.w, old.rect.h),
                (new.rect.x, new.rect.y, new.rect.w, new.rect.h)
            );
        }
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert!(!state.preferences.wallpaper_cycle);
    }
}

#[test]
fn gradient_toggle_is_saved_stable_and_switches_bundled_and_uploaded_images() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::WallpaperGradient;
    state.scroll = 300.;
    let defaults: crate::model::Preferences = serde_json::from_str("{}").unwrap();
    assert!(defaults.wallpaper_gradient);
    for language in crate::i18n::Language::ALL {
        state.preferences.language = language;
        let before = controls(&state);
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert!(!state.preferences.wallpaper_gradient);
        assert_eq!(state.focus, Target::WallpaperGradient);
        assert_eq!(state.scroll, 300.);
        assert!(state.overlay.is_none() && state.subpage.is_none());
        let rendered = scene(&state);
        assert!(
            matches!(&rendered.draw[0], Draw::Image(path, ..) if path.ends_with("dunes-plain.jpg"))
        );
        for (old, new) in before.iter().zip(&rendered.controls) {
            assert_eq!(old.target, new.target);
            assert_eq!(
                (old.rect.x, old.rect.y, old.rect.w),
                (new.rect.x, new.rect.y, new.rect.w)
            );
        }
        let saved = serde_json::to_string(&state.preferences).unwrap();
        let restored: crate::model::Preferences = serde_json::from_str(&saved).unwrap();
        assert!(!restored.wallpaper_gradient);
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert!(
            matches!(&scene(&state).draw[0], Draw::Image(path, ..) if path.ends_with("dunes.jpg"))
        );
    }
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Background(2));
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperGradient);
    let item = custom(1);
    state.preferences.background = item.id.clone();
    state.set_wallpapers(vec![item]);
    state.activate();
    assert!(
        matches!(&scene(&state).draw[0], Draw::Image(path, ..) if path.ends_with("original.jpg"))
    );
    state.wallpapers[0].has_original = false;
    assert!(
        matches!(&scene(&state).draw[0], Draw::Image(path, ..) if path.ends_with("wallpaper.jpg"))
    );
}

#[test]
fn cycling_uses_all_uploaded_ids_but_not_deleted_or_pending_deletion_entries() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    let first = custom(1);
    let second = custom(2);
    state.set_wallpapers(vec![first.clone(), second.clone()]);
    state.preferences.background = "northern-lights".into();
    state.deleting_wallpaper = Some(first.id.clone());
    assert!(state.cycle_wallpaper(100));
    assert_eq!(state.preferences.background, second.id);
    assert!(!state.can_delete_background(&second.id));
    assert!(state.previous_background.is_none());
    assert!(state.cycle_wallpaper(200));
    assert_eq!(state.preferences.background, "slate");
    state.deleting_wallpaper = None;
    state.preferences.background = "northern-lights".into();
    assert!(state.cycle_wallpaper(300));
    assert_eq!(state.preferences.background, first.id);
    state.preferences.background = "dunes".into();
    state.preferences.deleted_backgrounds = vec!["san-francisco".into()];
    assert!(state.cycle_wallpaper(400));
    assert_eq!(state.preferences.background, "summit");
}

#[test]
fn automatic_changes_preserve_manual_splash_across_saves_and_restart_and_manual_selection_resets_time()
 {
    let mut state = ready();
    state.preferences.background = "forest".into();
    state.preferences.wallpaper_cycle = true;
    state.preferences.wallpaper_changed = 100;
    assert!(!crate::wallpaper_cycle::due(true, 100, 100));
    assert!(crate::wallpaper_cycle::startup_due(
        true,
        true,
        true,
        std::time::Duration::ZERO
    ));
    assert!(state.cycle_wallpaper(100));
    assert_eq!(state.background_fade, crate::state::CYCLE_FADE);
    assert_eq!(state.preferences.splash_background(), "forest");
    state.preferences.show_seconds = !state.preferences.show_seconds;
    let mut preferences: Preferences =
        serde_json::from_value(serde_json::to_value(&state.preferences).unwrap()).unwrap();
    preferences.sanitize();
    assert_eq!(preferences.splash_background(), "forest");
    let mut restored = State::new(preferences);
    assert!(restored.cycle_wallpaper(200));
    assert_eq!(restored.preferences.splash_background(), "forest");
    restored.open(2);
    restored.section = 1;
    restored.focus = Target::Background(0);
    restored.activate();
    assert_eq!(restored.activate(), vec![Effect::Save]);
    assert_eq!(restored.preferences.background, "slate");
    assert_eq!(restored.preferences.splash_background(), "slate");
    assert_eq!(restored.background_fade, crate::state::BACKGROUND_FADE);
    assert!(restored.preferences.wallpaper_changed > 200);
}

#[test]
fn former_exclusions_are_ignored_and_only_the_on_off_state_is_saved() {
    let preferences: Preferences = serde_json::from_value(serde_json::json!({
        "wallpaper_cycle": "hourly", "wallpaper_skip": ["san-francisco"], "background": "dunes"
    }))
    .unwrap();
    let mut state = State::new(preferences);
    assert!(state.preferences.wallpaper_cycle);
    assert!(state.cycle_wallpaper(100));
    assert_eq!(state.preferences.background, "san-francisco");
    let saved = serde_json::to_value(state.preferences).unwrap();
    assert_eq!(saved["wallpaper_cycle"], true);
    assert!(saved.get("wallpaper_skip").is_none());
}

#[test]
fn upload_is_modal_and_close_restores_its_exact_parent() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::Background(state.background_count() - 1);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::WallpaperUpload)));
    assert_eq!(state.focus, Target::CloseDialog);
    let modal = scene(&state);
    assert!(modal.modal.is_some());
    assert_eq!(modal.controls.len(), 1);
    assert_eq!(modal.controls[0].target, Target::CloseDialog);
    state.direction(Direction::Right, &modal.controls);
    assert_eq!(state.focus, Target::CloseDialog);
    state.activate();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::UploadWallpaper);
    assert_eq!(state.section, 1);
    // The upload tile is the gallery's last: up is the tile above it.
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(
        state.focus,
        Target::Background(state.background_count() - 4)
    );
}

#[test]
fn upload_stays_active_without_desktop_focus_but_requires_tv_focus() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::UploadWallpaper;
    state.activate();

    for webos in [false, true] {
        assert!(crate::wallpaper_upload_active(&state, webos, true, true));
        assert_eq!(
            crate::wallpaper_upload_active(&state, webos, false, true),
            !webos
        );
        for focused in [false, true] {
            assert!(!crate::wallpaper_upload_active(
                &state, webos, focused, false
            ));
        }
    }

    state.back();
    for webos in [false, true] {
        for focused in [false, true] {
            assert!(!crate::wallpaper_upload_active(
                &state, webos, focused, true
            ));
        }
    }
}

#[test]
fn upload_instructions_are_ordered_and_only_the_displayed_url_loses_its_slash() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::UploadWallpaper;
    state.activate();
    let url = "http://192.168.100.100:54321/AbCd1234/";
    state.wallpaper_upload = Some(Ok(crate::wallpapers::Session {
        url: url.into(),
        qr_size: 21,
        qr: vec![false; 21 * 21],
    }));
    for language in crate::i18n::Language::ALL {
        state.preferences.language = language;
        let modal = scene(&state);
        let labels: Vec<_> = modal
            .draw
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text(label, rect, ..) => Some((label.as_str(), rect)),
                _ => None,
            })
            .collect();
        let mut previous_bottom = 0.;
        for label in [
            language.t("wallpaperUploadHint"),
            language.t("wallpaperUploadBrowser"),
            url.trim_end_matches('/'),
            language.t("wallpaperUploadKeepOpen"),
            language.t("wallpaperUploadPrivate"),
            language.t("close"),
        ] {
            let (_, rect) = labels.iter().find(|(text, _)| *text == label).unwrap();
            assert!(rect.y >= previous_bottom);
            previous_bottom = rect.y + rect.h;
        }
        assert!(!labels.iter().any(|(label, _)| *label == url));
        assert_eq!(modal.controls.len(), 1);
        assert_eq!(modal.controls[0].target, Target::CloseDialog);
        let panel = modal.modal.unwrap();
        let close = modal.controls[0].rect;
        assert!(panel.y + panel.h - close.y - close.h >= 40.);
    }
    let session = state.wallpaper_upload.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(session.url, url);
}

#[test]
fn upload_tile_names_itself_inside_the_tile_in_every_language() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    for language in crate::i18n::Language::ALL {
        state.preferences.language = language;
        let rendered = scene(&state);
        let upload = rendered
            .controls
            .iter()
            .find(|c| c.target == Target::UploadWallpaper)
            .unwrap();
        assert!(upload.scrollable);
        let title = rendered
            .draw
            .iter()
            .find_map(|draw| match draw {
                Draw::Text(label, rect, ..) if label == language.t("uploadWallpaper") => Some(rect),
                _ => None,
            })
            .unwrap();
        assert!(title.x >= upload.rect.x + 24.);
        assert!(title.x + title.w <= upload.rect.x + upload.rect.w - 24.);
        assert!(
            title.y > upload.rect.y + upload.rect.h / 2.,
            "the label sits under the icon"
        );
    }
}

#[test]
fn gallery_scrolls_under_fixed_switches_through_a_mask() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    // A page opens at its content; here, the gallery's, so the sidebar is a column of icons.
    state.focus = Target::WallpaperGradient;
    state.set_wallpapers((1..=8).map(custom).collect());
    let before = controls(&state);
    state.wheel_scroll(&before, Some((1000., 600.)), 0, -3);
    assert!(state.scroll > 0.);
    let scrolled = scene(&state);
    for target in [
        Target::WallpaperCycle,
        Target::WallpaperGradient,
        Target::Sidebar(0),
        Target::Sidebar(1),
        Target::Sidebar(crate::state::SYSTEM_SECTION),
    ] {
        let old = before.iter().find(|c| c.target == target).unwrap();
        let new = scrolled
            .controls
            .iter()
            .find(|c| c.target == target)
            .unwrap();
        assert_eq!(
            (old.rect.x, old.rect.y, old.rect.w, old.rect.h),
            (new.rect.x, new.rect.y, new.rect.w, new.rect.h)
        );
        assert!(!new.scrollable);
    }
    for target in [Target::Background(3), Target::UploadWallpaper] {
        let old = before.iter().find(|c| c.target == target).unwrap();
        let new = scrolled
            .controls
            .iter()
            .find(|c| c.target == target)
            .unwrap();
        assert_eq!(old.rect.y - new.rect.y, state.scroll);
    }
    let viewport = state.backgrounds_viewport();
    assert!(
        scrolled
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Clip(Some(rect)) if rect.y == viewport.y))
    );
    let last_card = scrolled
        .draw
        .iter()
        .rposition(|draw| {
            matches!(draw,
                Draw::Image(_, rect, ..) if rect.w == 390. && rect.h == 219.
            )
        })
        .unwrap();
    // The sidebar, a column of icons while the gallery has focus, is drawn last of all.
    for key in ["wallpaperGradient", "wallpaperCycle"] {
        let label = state.preferences.language.t(key);
        let fixed = scrolled
            .draw
            .iter()
            .position(|draw| {
                matches!(draw,
                    Draw::Text(text, ..) if text == label
                )
            })
            .unwrap();
        assert!(fixed > last_card, "{key} must be drawn above the gallery");
    }
    let scroll = state.scroll;
    state.wheel_scroll(&scrolled.controls, Some((300., 180.)), 0, -3);
    assert_eq!(state.scroll, scroll);
}

#[test]
fn scrolled_backgrounds_cannot_capture_upload_or_sidebar_pointer_hits() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    // A page opens at its content; here, the gallery's, so the sidebar is a column of icons.
    state.focus = Target::WallpaperGradient;
    state.scroll = 150.;
    let gallery = controls(&state);
    let upload = gallery
        .iter()
        .find(|c| c.target == Target::UploadWallpaper)
        .unwrap()
        .rect;
    let (x, y) = upload.center();
    assert_eq!(
        state.hit_test(&gallery, x, y),
        Some(Target::UploadWallpaper)
    );
    assert_eq!(state.hit_test(&gallery, 1000., 200.), None);
    assert_eq!(
        state.hit_test(&gallery, 118., 256.),
        Some(Target::Sidebar(1))
    );
    assert_eq!(
        state.hit_test(&gallery, 400., 260.),
        Some(Target::Background(0))
    );
}

#[test]
fn remote_gallery_focus_stays_below_the_fixed_switches() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.set_wallpapers((1..=9).map(custom).collect());
    state.focus = Target::Background(2);
    for _ in 0..5 {
        state.direction(Direction::Down, &controls(&state));
    }
    assert!(state.scroll > 0.);
    for _ in 0..4 {
        state.direction(Direction::Up, &controls(&state));
        if matches!(state.focus, Target::Background(_)) {
            let gallery = controls(&state);
            let focused = gallery.iter().find(|c| c.target == state.focus).unwrap();
            let viewport = state.backgrounds_viewport();
            assert!(focused.rect.y >= viewport.y);
            assert!(focused.rect.y + focused.rect.h <= viewport.y + viewport.h);
        }
    }
    assert_eq!(state.focus, Target::Background(2));
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperGradient);
}

#[test]
fn custom_gallery_navigation_reveals_whole_cards_in_incomplete_rows() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    // With Apple TV last, twelve cards and the upload tile: the last row holds it alone.
    state.set_wallpapers(vec![custom(1), custom(2)]);
    state.focus = Target::Background(10);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::Background(11));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Background(8));
    state.focus = Target::Background(10);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    let focused = controls(&state)
        .into_iter()
        .find(|c| c.target == state.focus)
        .unwrap();
    let viewport = state.content_viewport();
    assert!(focused.rect.y >= viewport.y);
    assert!(focused.rect.y + focused.rect.h <= viewport.y + viewport.h);
}

#[test]
fn upload_discovery_does_not_apply_and_custom_selection_uses_immutable_paths() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    // A page opens at its content; here, the gallery's, so the sidebar is a column of icons.
    state.focus = Target::WallpaperGradient;
    let item = custom(1);
    state.set_wallpapers(vec![item.clone()]);
    assert_eq!(state.preferences.background, "dunes");
    let gallery = scene(&state);
    assert!(
        gallery
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Image(path, ..) if path == &item.thumbnail()))
    );
    state.focus = Target::Background(BACKGROUNDS.len());
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::UseBackground);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.background, item.id);
    assert!(
        scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::TintedImage(path, ..) if path == &item.image()))
    );
    state.previous_background = None;
    assert!(matches!(&scene(&state).draw[0], Draw::Image(path, ..) if path == &item.image()));
    let mut restored = State::new(
        serde_json::from_slice(&serde_json::to_vec(&state.preferences).unwrap()).unwrap(),
    );
    restored.preferences.sanitize();
    assert_eq!(restored.preferences.background, item.id);
    // Missing uploads fall back without overwriting the saved selection.
    assert!(
        matches!(&scene(&restored).draw[0], Draw::Image(path, ..) if path.ends_with("wallpapers/dunes.jpg"))
    );
    restored.set_wallpapers(vec![item.clone()]);
    assert!(matches!(&scene(&restored).draw[0], Draw::Image(path, ..) if path == &item.image()));
}

#[test]
fn catalog_updates_keep_the_focused_image_and_pointer_geometry_stationary() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    let item = custom(1);
    state.set_wallpapers(vec![item.clone()]);
    state.focus = Target::Background(9);
    state.set_wallpapers(vec![custom(2), item.clone()]);
    assert_eq!(state.focus, Target::Background(10));
    assert_eq!(state.background_id(10), Some(item.id.as_str()));
    let scroll = state.scroll;
    let before = controls(&state)
        .into_iter()
        .find(|c| c.target == Target::Background(10))
        .unwrap();
    state.pointer_focus(Target::Background(10));
    assert_eq!(state.scroll, scroll);
    let after = controls(&state)
        .into_iter()
        .find(|c| c.target == Target::Background(10))
        .unwrap();
    assert_eq!((before.rect.x, before.rect.y), (after.rect.x, after.rect.y));
}

#[test]
fn background_popup_is_modal_lifts_the_same_thumbnail_and_restores_its_parent() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::Background(0);
    let before = controls(&state)
        .into_iter()
        .find(|c| c.target == state.focus)
        .unwrap()
        .rect;
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::UseBackground);
    assert!(matches!(
        state.overlay,
        Some(Overlay::BackgroundOptions { .. })
    ));
    let popup = scene(&state);
    assert!(popup.modal.is_some());
    assert_eq!(
        popup
            .controls
            .iter()
            .map(|c| c.target.clone())
            .collect::<Vec<_>>(),
        vec![Target::UseBackground, Target::DeleteBackground]
    );
    assert!(popup.draw.iter().any(|draw| matches!(draw, Draw::Image(_, rect, ..) if rect.x == before.x && rect.y == before.y && rect.w == before.w && rect.h == before.h)));
    state.direction(Direction::Down, &popup.controls);
    assert_eq!(state.focus, Target::DeleteBackground);
    state.activate();
    assert_eq!(state.focus, Target::CancelDelete);
    assert!(matches!(
        state.overlay,
        Some(Overlay::BackgroundDelete { .. })
    ));
    state.activate();
    assert_eq!(state.focus, Target::DeleteBackground);
    state.back();
    assert_eq!(state.focus, Target::Background(0));
    assert!(state.overlay.is_none());
}

#[test]
fn selected_background_has_no_delete_action_and_rejects_forced_confirmation() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    let selected = state.preferences.background.clone();
    let index = (0..state.background_count())
        .find(|&i| state.background_id(i) == Some(&selected))
        .unwrap();
    state.focus = Target::Background(index);
    state.options();
    assert_eq!(scene(&state).controls.len(), 1);
    assert!(!state.can_delete_background(&selected));
    state.focus = Target::DeleteBackground;
    state.activate();
    assert!(matches!(
        state.overlay,
        Some(Overlay::BackgroundOptions { .. })
    ));
    state.overlay = Some(Overlay::BackgroundDelete {
        id: selected.clone(),
        restore: Target::Background(index),
    });
    state.focus = Target::ConfirmDelete;
    assert!(state.activate().is_empty());
    assert_eq!(state.preferences.background, selected);
    assert!(state.preferences.deleted_backgrounds.is_empty());
}

#[test]
fn every_unselected_bundled_background_can_be_removed_and_stays_removed_after_reload() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    for (id, _) in BACKGROUNDS {
        if *id == state.preferences.background {
            continue;
        }
        let index = (0..state.background_count())
            .find(|&i| state.background_id(i) == Some(id))
            .unwrap();
        state.focus = Target::Background(index);
        state.activate();
        state.focus = Target::DeleteBackground;
        state.activate();
        state.focus = Target::ConfirmDelete;
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert!(state.overlay.is_none());
        assert!(!(0..state.background_count()).any(|i| state.background_id(i) == Some(id)));
        assert!(controls(&state).iter().any(|c| c.target == state.focus));
    }
    // Apple TV is not packaged; it stays until removed itself.
    assert_eq!(state.background_count(), 2);
    let mut preferences: crate::model::Preferences =
        serde_json::from_slice(&serde_json::to_vec(&state.preferences).unwrap()).unwrap();
    preferences.sanitize();
    let restored = State::new(preferences);
    assert_eq!(restored.background_count(), 2);
    assert_eq!(restored.background_id(0), Some("dunes"));
    assert_eq!(restored.background_id(1), Some(crate::aerials::ID));
}

#[test]
fn uploaded_deletion_is_single_flight_handles_failures_and_keeps_unrelated_focus() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    let lake = custom(1);
    let mountain = custom(2);
    state.set_wallpapers(vec![lake.clone(), mountain.clone()]);
    state.focus = Target::Background(BACKGROUNDS.len());
    state.activate();
    state.focus = Target::DeleteBackground;
    state.activate();
    state.focus = Target::ConfirmDelete;
    assert_eq!(
        state.activate(),
        vec![Effect::DeleteWallpaper(lake.id.clone())]
    );
    state.focus = Target::ConfirmDelete;
    assert!(state.activate().is_empty());
    state.wallpaper_deleted(mountain.id.clone(), Ok(()));
    assert_eq!(state.wallpapers.len(), 2);
    state.wallpaper_deleted(lake.id.clone(), Err("read-only".into()));
    assert_eq!(state.wallpapers.len(), 2);
    assert!(state.deleting_wallpaper.is_none());
    state.focus = Target::ConfirmDelete;
    assert_eq!(
        state.activate(),
        vec![Effect::DeleteWallpaper(lake.id.clone())]
    );
    state.back();
    state.focus = Target::UseBackground;
    assert!(state.activate().is_empty()); // Cannot apply an image being deleted.
    assert_eq!(state.preferences.background, "dunes");
    state.back();
    state.focus = Target::Background(BACKGROUNDS.len() + 1);
    state.wallpaper_deleted(lake.id, Ok(()));
    assert_eq!(state.focus, Target::Background(BACKGROUNDS.len()));
    assert_eq!(
        state.background_id(BACKGROUNDS.len()),
        Some(mountain.id.as_str())
    );
    assert!(state.overlay.is_none());
}

#[test]
fn filtered_bundled_entries_do_not_misidentify_custom_thumbnails_or_popup_parents() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.preferences.deleted_backgrounds = BACKGROUNDS
        .iter()
        .filter(|(id, _)| *id != "dunes")
        .map(|(id, _)| (*id).into())
        .collect();
    let lake = custom(1);
    state.set_wallpapers(vec![lake.clone()]);
    state.focus = Target::Background(1);
    assert!(
        scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Image(path, ..) if path == &lake.thumbnail()))
    );
    state.options();
    state.set_wallpapers(vec![custom(2), lake.clone()]);
    state.back();
    assert_eq!(state.focus, Target::Background(2));
    state.options();
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.background, lake.id);
}

#[test]
fn removed_popup_entry_closes_safely_and_preference_sanitizing_protects_the_selection() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.set_wallpapers(vec![custom(1)]);
    state.focus = Target::Background(BACKGROUNDS.len());
    state.options();
    state.set_wallpapers(vec![]);
    assert!(state.overlay.is_none());
    assert!(controls(&state).iter().any(|c| c.target == state.focus));
    state.preferences.deleted_backgrounds = vec![
        "dunes".into(),
        "../bad".into(),
        "forest".into(),
        "forest".into(),
    ];
    state.preferences.sanitize();
    assert!(
        !state
            .preferences
            .deleted_backgrounds
            .iter()
            .any(|id| id == "dunes" || id == "../bad")
    );
    assert!(state.preferences.deleted_backgrounds.len() <= 1);
}

#[test]
fn a_dynamic_wallpaper_shows_the_frame_for_the_sun_and_fades_between_frames() {
    use crate::dynamic_wallpaper::Frame;
    let mut state = ready();
    let mut item = custom(7);
    item.frames = vec![
        Frame {
            altitude: Some(-30.),
            azimuth: Some(0.),
            time: None,
        },
        Frame {
            altitude: Some(60.),
            azimuth: Some(180.),
            time: None,
        },
    ];
    state.set_wallpapers(vec![item.clone()]);
    state.preferences.background = item.id.clone();
    // Kyiv: midday in June, then the night after.
    state.preferences.weather_location = Some(crate::weather::Location {
        name: "Kyiv".into(),
        region: String::new(),
        country: "Ukraine".into(),
        latitude: 50.45,
        longitude: 30.52,
        id: None,
    });
    let noon = time::OffsetDateTime::from_unix_timestamp(1_782_036_000).unwrap();
    state.sync_dynamic_wallpaper(noon);
    assert_eq!(state.background_key(), format!("{}#1", item.id));
    assert!(
        state.previous_background.is_none(),
        "the first frame does not fade in"
    );
    state.sync_dynamic_wallpaper(noon + time::Duration::seconds(10));
    assert_eq!(state.background_key(), format!("{}#1", item.id));
    state.sync_dynamic_wallpaper(noon + time::Duration::hours(12));
    assert_eq!(state.background_key(), format!("{}#0", item.id));
    assert_eq!(
        state
            .previous_background
            .as_ref()
            .map(|(key, _)| key.clone()),
        Some(format!("{}#1", item.id))
    );
    // Another wallpaper drops the frame.
    state.preferences.background = "dunes".into();
    state.sync_dynamic_wallpaper(noon);
    assert_eq!(state.background_key(), "dunes");
}

fn aerial(index: usize) -> Wallpaper {
    let id = &crate::aerials::catalog().pictures[index].id;
    Wallpaper {
        id: id.clone(),
        directory: PathBuf::from(format!("/preview/aerials/{id}")),
        has_original: true,
        frames: Vec::new(),
    }
}

fn places() -> usize {
    crate::aerials::catalog().pictures.len()
}

/// Chooses a gallery entry the way the remote does: OK on its card, then Use.
fn choose(state: &mut State, id: &str) {
    state.open(2);
    state.section = 1;
    let index = (0..state.background_count())
        .position(|index| state.background_id(index) == Some(id))
        .unwrap();
    state.focus = Target::Background(index);
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::UseBackground);
    assert_eq!(state.activate(), vec![Effect::Save]);
}

#[test]
fn apple_tv_is_the_last_gallery_entry_and_shows_its_place() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    // A page opens at its content; here, the gallery's, so the sidebar is a column of icons.
    state.focus = Target::WallpaperGradient;
    state.set_wallpapers(vec![custom(1)]);
    let last = state.background_count() - 1;
    assert_eq!(state.background_id(last), Some(crate::aerials::ID));
    assert_eq!(
        state.background_id(BACKGROUNDS.len()),
        Some(custom(1).id.as_str())
    );
    let place = aerial(state.preferences.aerials.current(places()));
    state.aerial_ready(place.clone());
    state.preferences.language = crate::i18n::Language::Ru;
    let gallery = scene(&state);
    assert!(
        gallery
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Image(path, ..) if path == &place.thumbnail()))
    );
    let name = crate::aerials::catalog().pictures[state.preferences.aerials.current(places())]
        .name(crate::i18n::Language::Ru);
    let texts: Vec<_> = gallery
        .draw
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text(text, ..) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&crate::aerials::LABEL), "{texts:?}");
    assert!(texts.contains(&name), "{texts:?}");
}

#[test]
fn choosing_apple_tv_keeps_the_last_picture_until_its_still_arrives_then_fades_to_it() {
    let mut state = ready();
    choose(&mut state, "forest");
    state.previous_background = None;
    choose(&mut state, crate::aerials::ID);
    assert_eq!(state.preferences.background, crate::aerials::ID);
    // The splash stays a packaged picture, and the rotation starts now.
    assert_eq!(state.preferences.splash_background(), "forest");
    let rotation = state.preferences.aerials;
    assert!(rotation.changed > 0);
    assert!(std::mem::take(&mut state.aerials_request));
    let current = aerial(rotation.current(places()));
    let next = aerial(rotation.next(places()));
    assert_eq!(
        state.aerial_ids(),
        vec![current.id.clone(), next.id.clone()]
    );
    // Nothing on the TV yet: the last picture stays.
    assert_eq!(state.background_key(), "forest");
    state.previous_background = None;
    state.aerial_ready(next.clone());
    assert_eq!(state.background_key(), "forest");
    assert!(state.previous_background.is_none());
    state.aerial_ready(current.clone());
    assert_eq!(state.background_key(), format!("aerials#{}", current.id));
    assert!(matches!(&state.previous_background, Some((key, _)) if key == "forest"));
    state.previous_background = None;
    assert!(matches!(&scene(&state).draw[0], Draw::Image(path, ..) if path == &current.image()));
    state.preferences.wallpaper_gradient = false;
    assert!(
        matches!(&scene(&state).draw[0], Draw::Image(path, ..) if path == &current.unshaded_image())
    );
}

#[test]
fn apple_tv_moves_on_every_fifteen_minutes_once_the_next_still_is_on_the_tv() {
    let mut state = ready();
    state.preferences.background = crate::aerials::ID.into();
    state.preferences.aerials = crate::aerials::Rotation::start(1000);
    let rotation = state.preferences.aerials;
    let current = aerial(rotation.current(places()));
    let next = aerial(rotation.next(places()));
    state.aerial_ready(current.clone());
    state.aerials_request = false;
    assert!(!state.advance_aerials(1000 + crate::aerials::INTERVAL - 1));
    // Due, but the next still has not arrived (offline): the place stays.
    assert!(!state.advance_aerials(1000 + crate::aerials::INTERVAL));
    assert_eq!(state.preferences.aerials, rotation);
    state.aerial_ready(next.clone());
    assert!(state.advance_aerials(1000 + crate::aerials::INTERVAL));
    assert_eq!(
        state.preferences.aerials.changed,
        1000 + crate::aerials::INTERVAL
    );
    assert_eq!(state.background_key(), format!("aerials#{}", next.id));
    assert!(
        matches!(&state.previous_background, Some((key, _)) if *key == format!("aerials#{}", current.id))
    );
    assert!(state.aerials_request);
    let after = aerial(state.preferences.aerials.next(places()));
    // The still fading out stays until the next change.
    assert_eq!(state.aerial_ids(), vec![next.id, after.id, current.id]);
}

#[test]
fn rotating_through_the_gallery_skips_apple_tv_which_rotates_on_its_own() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    let lake = custom(1);
    state.set_wallpapers(vec![lake.clone()]);
    state.preferences.background = lake.id.clone();
    assert!(state.cycle_wallpaper(100));
    assert_eq!(state.preferences.background, "slate");
    state.preferences.background = crate::aerials::ID.into();
    assert!(!state.cycle_wallpaper(200));
    assert_eq!(state.preferences.background, crate::aerials::ID);
}

#[test]
fn apple_tv_survives_a_restart_and_cannot_be_removed_from_the_gallery() {
    let mut preferences = Preferences {
        background: crate::aerials::ID.into(),
        splash_background: Some(crate::aerials::ID.into()),
        ..Preferences::default()
    };
    preferences.sanitize();
    assert_eq!(preferences.background, crate::aerials::ID);
    assert_eq!(preferences.splash_background(), crate::aerials::ID);
    assert_eq!(preferences.splash_background, None);
    // Removed by an earlier build: it comes back.
    let mut preferences = Preferences {
        deleted_backgrounds: vec![crate::aerials::ID.into(), "summit".into()],
        ..Preferences::default()
    };
    preferences.sanitize();
    assert_eq!(preferences.deleted_backgrounds, vec!["summit".to_owned()]);
    let mut state = ready();
    choose(&mut state, "summit");
    let index = state.background_count() - 1;
    assert_eq!(state.background_id(index), Some(crate::aerials::ID));
    assert!(!state.can_delete_background(crate::aerials::ID));
    state.focus = Target::Background(index);
    state.activate();
    let controls: Vec<_> = scene(&state)
        .controls
        .into_iter()
        .map(|c| c.target)
        .collect();
    assert_eq!(controls, vec![Target::UseBackground]);
    state.focus = Target::DeleteBackground;
    state.activate();
    assert!(matches!(
        state.overlay,
        Some(Overlay::BackgroundOptions { .. })
    ));
    state.overlay = Some(Overlay::BackgroundDelete {
        id: crate::aerials::ID.into(),
        restore: Target::Background(index),
    });
    state.focus = Target::ConfirmDelete;
    assert!(state.activate().is_empty());
    assert!(state.preferences.deleted_backgrounds.is_empty());
    assert_eq!(state.background_id(index), Some(crate::aerials::ID));
}

#[test]
fn apple_tv_in_use_can_be_moved_to_its_next_place_at_once() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    choose(&mut state, crate::aerials::ID);
    let rotation = state.preferences.aerials;
    let current = aerial(rotation.current(places()));
    let next = aerial(rotation.next(places()));
    state.aerial_ready(current.clone());
    state.aerial_ready(next.clone());
    state.aerials_request = false;
    let now = rotation.changed + 60;
    assert!(!state.advance_aerials(now));
    let index = state.background_count() - 1;
    state.focus = Target::Background(index);
    state.activate();
    let controls: Vec<_> = scene(&state)
        .controls
        .into_iter()
        .map(|c| c.target)
        .collect();
    assert_eq!(controls, vec![Target::UseBackground, Target::NextAerial]);
    state.focus = Target::NextAerial;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.overlay.is_none());
    assert!(state.aerials_request);
    assert!(state.advance_aerials(now));
    assert_eq!(state.background_key(), format!("aerials#{}", next.id));
    assert_eq!(state.preferences.aerials.changed, now);
    // The fifteen minutes start again from the change.
    assert!(!state.advance_aerials(now + crate::aerials::INTERVAL - 1));
    // Not the background: there is no next place to move to.
    choose(&mut state, "summit");
    state.focus = Target::Background(index);
    state.activate();
    let controls: Vec<_> = scene(&state)
        .controls
        .into_iter()
        .map(|c| c.target)
        .collect();
    assert_eq!(controls, vec![Target::UseBackground]);
    state.focus = Target::NextAerial;
    assert!(state.activate().is_empty());
    assert_eq!(state.preferences.background, "summit");
}

fn fade_end(state: &State, wallpapers: view::Wallpapers) -> view::Scene {
    let assets = view::Assets {
        root: std::path::PathBuf::new(),
        version: "test".into(),
        metrics: std::collections::BTreeMap::new(),
        artwork: crate::app_artwork::Service::new(false),
    };
    view::scene_on(state, &assets, time::OffsetDateTime::UNIX_EPOCH, wallpapers)
}

#[test]
fn a_fade_the_renderer_holds_waits_on_the_old_wallpaper_then_runs_whole() {
    let mut state = ready();
    state.hold_fades = true;
    state.preferences.background = "forest".into();
    assert!(state.cycle_wallpaper(100));
    assert!(state.wallpaper_waiting);
    // The new wallpaper is not drawn yet, so the renderer need not decode it mid-frame.
    let held = scene(&state);
    assert!(matches!(&held.draw[1], Draw::TintedImage(_, _, _, _, tint) if tint[3] == 0.));
    assert_eq!(held.wallpaper_fade, Some(0.));
    // However long it waits, the fade does not run out.
    state.previous_background.as_mut().unwrap().1 = Instant::now() - Duration::from_secs(10);
    state.animate(0.016);
    assert!(state.previous_background.is_some());
    // Released, it runs from its start.
    state.release_wallpaper_fade();
    assert!(!state.wallpaper_waiting);
    assert!(
        scene(&state)
            .wallpaper_fade
            .is_some_and(|alpha| alpha < 0.1)
    );
    // Without a renderer to hold them, fades start at once.
    let mut state = ready();
    state.preferences.background = "forest".into();
    assert!(state.cycle_wallpaper(100));
    assert!(!state.wallpaper_waiting);
}

#[test]
fn a_fade_has_two_ends_each_on_one_wallpaper() {
    let mut state = ready();
    state.preferences.background = "forest".into();
    let before = scene(&state);
    assert!(state.cycle_wallpaper(100));
    let after = state.preferences.background.clone();
    let fading = scene(&state);
    assert!(fading.wallpaper_fade.is_some());
    assert_eq!(fading.backdrop.len(), 2);
    let from = fade_end(&state, view::Wallpapers::Previous);
    let to = fade_end(&state, view::Wallpapers::Current);
    for end in [&from, &to] {
        assert!(end.wallpaper_fade.is_none());
        assert_eq!(end.backdrop.len(), 1);
        assert!(!matches!(&end.draw[1], Draw::TintedImage(..)));
    }
    // The fade starts from the screen as it was.
    assert_eq!(from.backdrop, before.backdrop);
    assert!(matches!(&to.draw[0], Draw::Image(path, ..) if path.contains(&after)));
}

#[test]
fn a_held_fade_waits_for_what_else_moves_but_not_for_itself() {
    let mut state = ready();
    state.hold_fades = true;
    state.preferences.background = "forest".into();
    assert!(!state.animating_besides_wallpaper());
    assert!(state.cycle_wallpaper(100));
    assert!(state.animating());
    assert!(!state.animating_besides_wallpaper());
    // The menu opening over Home changes both of the fade's ends.
    state.menu_motion = Some((Instant::now(), true));
    assert!(state.animating_besides_wallpaper());
}
