//! Generated from the viewBoxes of `assets/ui-icons/cs-*.svg` (CS2's own icons, see
//! `assets/licenses/valve-cs2-icons.txt`): each icon's width over its height, sorted by name.
pub const ICONS: &[(&str, f32)] = &[
    ("cs-blind", 1.000),
    ("cs-bomb", 1.000),
    ("cs-headshot", 1.000),
    ("cs-noscope", 1.000),
    ("cs-smoke", 1.000),
    ("cs-suicide", 1.042),
    ("cs-wallbang", 1.000),
    ("cs-weapon-ak47", 3.054),
    ("cs-weapon-aug", 2.045),
    ("cs-weapon-awp", 3.135),
    ("cs-weapon-bayonet", 3.739),
    ("cs-weapon-bizon", 2.676),
    ("cs-weapon-c4", 1.000),
    ("cs-weapon-cz75a", 1.146),
    ("cs-weapon-deagle", 1.588),
    ("cs-weapon-decoy", 0.755),
    ("cs-weapon-elite", 1.538),
    ("cs-weapon-famas", 2.324),
    ("cs-weapon-fiveseven", 1.275),
    ("cs-weapon-flashbang", 0.847),
    ("cs-weapon-g3sg1", 2.995),
    ("cs-weapon-galilar", 2.676),
    ("cs-weapon-glock", 1.445),
    ("cs-weapon-hegrenade", 0.761),
    ("cs-weapon-hkp2000", 1.366),
    ("cs-weapon-incgrenade", 0.769),
    ("cs-weapon-inferno", 0.769),
    ("cs-weapon-knife", 3.336),
    ("cs-weapon-knife-bayonet", 3.739),
    ("cs-weapon-knife-butterfly", 3.868),
    ("cs-weapon-knife-canis", 3.982),
    ("cs-weapon-knife-cord", 5.022),
    ("cs-weapon-knife-css", 4.210),
    ("cs-weapon-knife-falchion", 3.407),
    ("cs-weapon-knife-flip", 3.918),
    ("cs-weapon-knife-gut", 3.595),
    ("cs-weapon-knife-gypsy-jackknife", 3.737),
    ("cs-weapon-knife-karambit", 2.400),
    ("cs-weapon-knife-m9-bayonet", 3.163),
    ("cs-weapon-knife-outdoor", 3.647),
    ("cs-weapon-knife-push", 2.703),
    ("cs-weapon-knife-skeleton", 3.457),
    ("cs-weapon-knife-stiletto", 5.084),
    ("cs-weapon-knife-survival-bowie", 4.036),
    ("cs-weapon-knife-t", 3.805),
    ("cs-weapon-knife-tactical", 3.167),
    ("cs-weapon-knife-ursus", 4.245),
    ("cs-weapon-knife-widowmaker", 2.353),
    ("cs-weapon-m249", 2.968),
    ("cs-weapon-m4a1", 2.698),
    ("cs-weapon-m4a1-silencer", 2.973),
    ("cs-weapon-m4a1-silencer-off", 2.973),
    ("cs-weapon-mac10", 1.432),
    ("cs-weapon-mag7", 2.005),
    ("cs-weapon-molotov", 0.769),
    ("cs-weapon-mp5sd", 2.135),
    ("cs-weapon-mp7", 1.495),
    ("cs-weapon-mp9", 1.941),
    ("cs-weapon-negev", 3.063),
    ("cs-weapon-nova", 2.982),
    ("cs-weapon-p250", 1.360),
    ("cs-weapon-p90", 2.005),
    ("cs-weapon-revolver", 1.503),
    ("cs-weapon-sawedoff", 3.138),
    ("cs-weapon-scar20", 3.054),
    ("cs-weapon-sg556", 2.667),
    ("cs-weapon-smokegrenade", 0.437),
    ("cs-weapon-ssg08", 3.014),
    ("cs-weapon-taser", 1.391),
    ("cs-weapon-tec9", 1.216),
    ("cs-weapon-trigger-hurt", 1.625),
    ("cs-weapon-ump45", 1.829),
    ("cs-weapon-usp-silencer", 2.052),
    ("cs-weapon-usp-silencer-off", 2.052),
    ("cs-weapon-world", 1.625),
    ("cs-weapon-xm1014", 3.032),
    ("cs-win-bomb", 1.000),
    ("cs-win-defuse", 1.000),
    ("cs-win-elimination", 1.000),
    ("cs-win-time", 1.000),
];

/// An icon's width over its height, if the game has it.
pub fn aspect(name: &str) -> Option<f32> {
    ICONS
        .binary_search_by(|(known, _)| known.cmp(&name))
        .ok()
        .map(|index| ICONS[index].1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn cs2_icons_are_sorted_for_lookup() {
        assert!(super::ICONS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(super::aspect("cs-weapon-m4a1-silencer").is_some());
        assert!(super::aspect("cs-weapon-zeus-new").is_none());
    }
}
