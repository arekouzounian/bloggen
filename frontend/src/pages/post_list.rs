use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::pagination::{PageQuery, Pagination};
use crate::Route;

const ITEMS_PER_PAGE: u64 = 10;
const TOTAL_PAGES: u64 = u64::MAX / ITEMS_PER_PAGE;

pub enum Msg {
    PageUpdated,
}

pub struct PostList {
    page: u64,
    _listener: LocationHandle,
}

fn current_page(ctx: &Context<PostList>) -> u64 {
    let location = ctx.link().location().unwrap();

    location.query::<PageQuery>().map(|it| it.page).unwrap_or(1)
}

impl Component for PostList {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let listener = ctx
            .link()
            .add_location_listener(link.callback(move |_| Msg::PageUpdated))
            .unwrap();

        Self {
            page: current_page(ctx),
            _listener: listener,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::PageUpdated => self.page = current_page(ctx),
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let page = self.page;

        html! {
            <div class="section container">
                <h1 class="title">{ "Posts" }</h1>
                <h2 class="subtitle">{ "this is just a copy-paste procedure" }</h2>
                { self.view_posts(ctx) }
                <Pagination
                    {page}
                    total_pages={TOTAL_PAGES}
                    route_to_page={Route::Posts}
                />

            </div>
        }
    }
}

impl PostList {
    fn view_posts(&self, _ctx: &Context<Self>) -> Html {
        // code to view all posts goes here
        // in the demo they populate with seeds
        // in this case, we want to grab posts by their IDs presumably

        // placeholder
        html! {
            <div>
                {"Posts should go here!"}
            </div>
        }
    }
}
