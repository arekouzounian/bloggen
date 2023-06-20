use yew::prelude::*;
// use yew_router::prelude::*;

pub struct Post {
    post: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub id: u64,
}

impl Component for Post {
    type Message = ();
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            post: String::new(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        !self.post.is_empty()
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <div class="content-container">
                    {self.view_content()}
                </div>
            </div>
        }
    }
}

impl Post {
    fn view_content(&self) -> Html {
        html! {
            <textarea>
                { self.post }
            </textarea>
        }
    }
}
