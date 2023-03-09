use crate::{CLIENT, console, graphql};
use crate::graphql::parts_query::PartsQueryParts;
use futures::FutureExt;
use std::collections::BTreeMap;
use std::rc::Rc;
use web_sys::{HtmlFormElement, HtmlInputElement};
use yew::prelude::*;
use yew::html::TargetCast;

// TODO: Handle errors (even when data is also returned
async fn get_parts() -> Vec<PartsQueryParts> {
    match graphql_client::reqwest::post_graphql::<graphql::PartsQuery, _>(
        &CLIENT,
        "http://localhost:8000/query",
        graphql::parts_query::Variables{}
    ).await {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                for e in errors {
                    console::log!("PartsQuery error: {}", e);
                }
                Vec::new()
            } else if let Some(resp_data) = resp.data {
                resp_data.parts
            } else {
                Vec::new()
            }
        },
        Err(e) => {
            console::log!("error getting parts: {}", e);
            Vec::new()
        }
    }
}

#[derive(Default, PartialEq)]
pub struct PartsMaps {
    parts: BTreeMap<Rc<str>, PartsQueryParts>,
    fams: BTreeMap<Rc<str>, Vec<Rc<str>>>,
}

impl PartsMaps {
    fn new(parts_vec: Vec<PartsQueryParts>) -> Self {
        let mut parts_maps = Self::default();
        for part in parts_vec {
            let ent = parts_maps.fams.entry(Rc::from(part.id.as_str()));
            let id = Rc::clone(ent.key());
            ent.or_insert(Vec::new());

            if let Some(pid) = part.parent_id.as_ref() {
                parts_maps.fams.entry(Rc::from(pid.as_str()))
                    .and_modify(|v| { v.push(Rc::clone(&id)) })
                    .or_insert(vec![Rc::clone(&id)]);
            }
            parts_maps.parts.insert(id, part);
        }
        parts_maps.fams.values_mut().for_each(|v| v.sort());
        parts_maps
    }
}

pub enum Msg {
    Parts(Vec<PartsQueryParts>),
    Login(graphql::login_mutation::Variables),
    Register(graphql::register_mutation::Variables),
}

pub struct App {
    parts_maps: Rc<PartsMaps>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let parts = get_parts();
        ctx.link().send_future(parts.map(Msg::Parts));
        Self {
            parts_maps: Rc::new(PartsMaps::default()),
        }
    }

    fn view(&self, _: &Context<Self>) -> Html {
        html! {
            <div id="app">
                if false {
                    { self.render_login() }
                } else {
                    { self.render_main() }
                }
            </div>
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Parts(parts) => {
                self.parts_maps = Rc::new(PartsMaps::new(parts));
                return true;
            }
            _ => return false,
        }
    }
}

impl App {
    fn render_login(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        // TODO: Possibly use batch_callback and handle None submitter differently
        let onsubmit = link.callback(|e: SubmitEvent| {
            e.prevent_default();
            let form: HtmlFormElement = e.target_unchecked_into();
            let submit_button = e.submitter()
                .expect("form event doesn't have submitter")
                .unchecked_into::<HtmlInputElement>();
            match submit_button.name().as_str() {
                "login" => Msg::Login(LoginVariables::default()),
                "register" => Msg::Register(RegisterVariables::default()),
                _ => unreachable!(),
            }
        })
        html! {
            <form id="login-register-div" {onsubmit}>
                <input type="email" size="40" placeholder="Email" required=true />
                <input type="password" size="40" placeholder="Password" required=true />
                <div id="logreg-button-div">
                    <input type="submit" name="login" value="Login" />
                    <input type="submit" name="register" value="Register" />
                </div>
            </form>
        }
    }

    fn render_main(&self) -> Html {
        let parts_maps = Rc::clone(&self.parts_maps);
        html! {
            <div id="main-div">
                <h1 style="text-align:center">{ "Hello, johnietrebus@gmail.com" }</h1>
                if parts_maps.parts.len() != 0 {
                    <ul>
                        {
                            parts_maps.parts.values()
                                .filter(|part| part.parent_id.is_none())
                                .map(|part| {
                                    html! {
                                        <Part
                                            key={part.id.as_str()}
                                            id={part.id.clone()}
                                            parts_maps={Rc::clone(&parts_maps)}
                                        />
                                    }
                                }).collect::<Html>()
                        }
                    </ul>
                } else {
                    <p>{ "No projects" }</p>
                }
            </div>
        }
    }
}

pub struct Part;

#[derive(Properties, PartialEq)]
pub struct PartProps {
    id: String,
    parts_maps: Rc<PartsMaps>,
}

impl Component for Part {
    type Message = ();
    type Properties = PartProps;

    fn create(_: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let id = &ctx.props().id;
        let parts_maps = Rc::clone(&ctx.props().parts_maps);
        let part = &parts_maps.parts[id.as_str()];
        let children = &parts_maps.fams[id.as_str()];
        html! {
            <li>
                if children.len() != 0 {
                    <details>
                        <summary>{ &part.name }<button>{ "+" }</button></summary>
                        <ul>
                        {
                            children.into_iter().map(|child_id| {
                                html! {
                                    <Part
                                        key={&**child_id}
                                        id={child_id.to_string()}
                                        parts_maps={Rc::clone(&parts_maps)}
                                    />
                                }
                            }).collect::<Html>()
                        }
                        </ul>
                    </details>
                } else {
                    <span>{ &part.name }<button>{ "+" }</button></span>
                }
            </li>
        }
    }
}
