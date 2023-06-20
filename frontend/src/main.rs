use std::thread::Scope;

use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod pages;

use pages::home::Home;
use pages::not_found::PageNotFound;
use pages::post::Post;
use pages::post_list::PostList;

#[derive(Routable, PartialEq, Eq, Clone, Debug)]
pub enum Route {
    #[at("/posts/:id")]
    Post { id: u64 },
    #[at("/posts")]
    Posts,
    #[at("/")]
    Home,
    #[not_found]
    #[at("/404")]
    NotFound,
}

pub enum Msg {
    ToggleNavbar,
}

pub struct App {
    navbar_active: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            navbar_active: true,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ToggleNavbar => {
                self.navbar_active = !self.navbar_active;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                {self.view_nav(ctx.link())}

                <main>
                    <Switch<Route> render={switch} />
                </main>
            </BrowserRouter>
        }
    }
}

impl App {
    fn view_nav(&self, link: &yew::html::Scope<Self>) -> Html {
        let Self { navbar_active, .. } = *self;

        let active_class = if !navbar_active { "is-active" } else { "" };

        html! {
                <nav class="navbar is-primary" role="navigation" aria-label="main navigation">
                    <div class="navbar-brand">
                        <h1 class="navbar-item is-size-3">{ "Yew Blog" }</h1>

                        <button class={classes!("navbar-burger", "burger", active_class)}
                            aria-label="menu" aria-expanded="false"
                            onclick={link.callback(|_| Msg::ToggleNavbar)}
                        >
                            <span aria-hidden="true"></span>
                            <span aria-hidden="true"></span>
                            <span aria-hidden="true"></span>
                        </button>
                    </div>
                    <div class={classes!("navbar-menu", active_class)}>
                        <div class="navbar-start">
                            <Link<Route> classes={classes!("navbar-item")} to={Route::Home}>
                                { "Home" }
                            </Link<Route>>
                            <Link<Route> classes={classes!("navbar-item")} to={Route::Posts}>
                                { "Posts" }
                            </Link<Route>>

                        </div>
                    </div>
                </nav>
        }
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Post { id } => {
            html! { <Post id={id} /> }
        }
        Route::Posts => {
            html! { <PostList /> }
        }
        Route::Home => {
            html! { <Home /> }
        }
        Route::NotFound => {
            html! { <PageNotFound /> }
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
