/*
 * A Lampa plugin that opens the card Luma asks for. On webOS, Lampa reads a card only from
 * "?card=" in its own address, while Luma's launch parameters ({card, media, source}) arrive in
 * PalmSystem.launchParams; this hands them to Lampa as its start page, and opens the card
 * whenever Luma relaunches a running Lampa.
 * Old webOS browsers run it, so it stays ES5.
 */
(function () {
    'use strict';

    function params() {
        try {
            return JSON.parse((window.PalmSystem && window.PalmSystem.launchParams) || '{}') || {};
            // Lampa runs on old TV browsers: no optional catch binding.
            // oxlint-disable-next-line no-unused-vars
        } catch (e) {
            return {};
        }
    }

    function link() {
        var p = params();
        var id = String(p.card || '');
        if (!/^[0-9]+$/.test(id)) return null;
        var source = p.source || 'cub';
        return {
            id: id,
            component: 'full',
            method: p.media === 'tv' ? 'tv' : 'movie',
            source: source,
            card: { id: id, source: source }
        };
    }

    function open() {
        var card = link();
        if (card) Lampa.Activity.push(card);
    }

    if (window.luma_cards) return;
    window.luma_cards = true;

    // Plugins load before Lampa picks its start page, which is the deep link when one is set;
    // pushing the card as well would put the start page over it.
    if (window.appready) open();
    else {
        var card = link();
        if (card) window.start_deep_link = card;
    }
    document.addEventListener('webOSRelaunch', open);
})();
