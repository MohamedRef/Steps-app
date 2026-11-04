//! The view switcher over a list (`parts::ViewBar`), on the References
//! page: the order menu and the layouts, kept on this device.

use super::App;
use crate::Screen;
use crate::i18n::views as tx;
use crate::parts::{Collection, Layout, Sort, ViewPick};
use crate::store::shared;

fn saved() -> ViewPick {
    shared().local_setting("view.refs").expect("the references view is saved")
}

#[test]
fn the_order_and_layout_are_picked_and_kept() {
    let mut app = App::open(Screen::Today);
    app.tap(crate::i18n::refs::TITLE);
    app.tap("view-sort-btn");
    app.assert_has(tx::SORT_TITLE);
    app.assert_has(tx::SORT_LEFT);
    app.tap(tx::SORT_NAME);
    assert_eq!(saved().sort, Sort::Name);
    assert!(!app.can_tap(tx::CLOSE_MENU), "picking closes the menu");
    // A tap beside the menu closes it and changes nothing.
    app.tap("view-sort-btn");
    app.tap(tx::CLOSE_MENU);
    assert!(!app.can_tap(tx::CLOSE_MENU));
    assert_eq!(saved().sort, Sort::Name);
    app.tap(tx::COMPACT);
    assert_eq!(saved(), ViewPick { layout: Layout::Compact, sort: Sort::Name, grouped: false });
}

#[test]
fn a_saved_order_the_list_no_longer_offers_falls_back() {
    const DEMO: Collection = Collection {
        key: "test.demo",
        layout: Layout::Cards,
        sorts: &[Sort::Book, Sort::Name],
        groups: false,
    };
    let mut app = App::open(Screen::Today);
    shared()
        .put_local_setting(
            "view.test.demo",
            &ViewPick { layout: Layout::List, sort: Sort::Due, grouped: true },
        )
        .unwrap();
    let pick = app.run(|| DEMO.pick());
    assert_eq!(pick, ViewPick { layout: Layout::List, sort: Sort::Book, grouped: false });
    // This run's choice wins over the saved one.
    app.run(|| DEMO.set(ViewPick { layout: Layout::Compact, sort: Sort::Name, grouped: false }));
    assert_eq!(app.run(|| DEMO.pick()).sort, Sort::Name);
}
