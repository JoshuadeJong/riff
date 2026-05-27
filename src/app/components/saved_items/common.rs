use crate::app::components::{CardListModel, CardListPage, CardListPageConfig};
use crate::app::dispatch::Worker;
use crate::app::AppEvent;

pub fn make_saved_page<M: CardListModel + 'static>(
    model: M,
    worker: Worker,
    config: CardListPageConfig,
    update_event: impl Fn(&AppEvent) -> bool + 'static,
) -> CardListPage<M> {
    CardListPage::new(model, worker, config, update_event)
}
