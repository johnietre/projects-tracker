// TODO: Make dtl_from_input return and uses better
// TODO: Make create/edit HTML reusable
// TODO: Create filter dropdown
// TODO: Fix filtering method: right now, a child would be displayed if the parent doesn't pass the
// filter
use crate::{
    console,
    graphql::{
        create_part_mutation::{
            CreatePartMutationCreatePart as CreatePartPart, Variables as CreatePartVars,
        },
        create_user_mutation::{CreateUserInput, Variables as CreateUserVars},
        delete_part_mutation::Variables as DeletePartVars,
        err_is_access, get_parts,
        login_user_mutation::{LoginUserInput, Variables as LoginUserVars},
        parts_query::PartsQueryParts,
        send_create_part, send_create_user, send_delete_part, send_login_user, send_logout_user,
        send_update_part,
        update_part_mutation::{
            UpdatePartMutationUpdatePart as UpdatePartPart, Variables as UpdatePartVars,
        },
        Part, PartialResult, DTL_INPUT_FMT, TZ,
    },
};
use chrono::prelude::*;
use futures::FutureExt;
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use web_sys::{
    Element, HtmlButtonElement, HtmlElement, HtmlFormElement, HtmlInputElement, HtmlSelectElement,
    HtmlTextAreaElement,
};
use yew::{html::TargetCast, prelude::*};

lazy_static::lazy_static! {
    static ref MIN_DT: DateTime<Local> = DateTime::from_utc(NaiveDateTime::MIN, *TZ);
    static ref MAX_DT: DateTime<Local> = DateTime::from_utc(NaiveDateTime::MAX, *TZ);
}

type PartsMap = BTreeMap<i64, Part>;

#[derive(Default, PartialEq)]
pub struct PartsMaps {
    // BTreeMap<id, PartsQueryParts>
    parts: PartsMap,
    // BTreeMap<parent_id, child_ids (if any)>
    fams: BTreeMap<i64, Vec<i64>>,
    // Projects = parts with no parents
    projects: Vec<i64>,

    sort_method: SortMethod,

    filter_method: FilterMethod,
    filtered_ids: Option<BTreeSet<i64>>,
}

impl PartsMaps {
    fn new(parts_vec: Vec<PartsQueryParts>) -> Self {
        let mut parts_maps = Self::default();
        for part in parts_vec {
            // TODO: Handle better?
            let id = part.id.parse().unwrap();
            parts_maps.fams.entry(id).or_insert(Vec::new());

            if let Some(pid_str) = part.parent_id.as_ref() {
                parts_maps
                    .fams
                    .entry(pid_str.parse().unwrap()) // TODO: Handle better?
                    .and_modify(|v| v.push(id))
                    .or_insert(vec![id]);
            } else {
                parts_maps.projects.push(id);
            }
            parts_maps.parts.insert(id, part.try_into().unwrap());
        }
        parts_maps
    }

    // Returns the part if a part with the id already existed
    fn add_part(&mut self, part: Part) -> Result<(), Part> {
        if self.parts.contains_key(&part.id) {
            return Err(part);
        }
        if let Some(pid) = part.parent_id {
            let children = self
                .fams
                .get_mut(&pid)
                .expect(&format!("missing fams pid: {}", pid));
            children.push(part.id);
            self.sort_method.sort(&self.parts, &mut *children);
        } else {
            self.projects.push(part.id);
            self.sort_projects();
        }
        if !self.filter_method.passes_filter(&part) {
            // Shouldn't be None if the part fails (i.e., the filter method isn't All)
            self.filtered_ids
                .as_mut()
                .expect("None filtered_ids with non-All method")
                .insert(part.id);
        }
        self.fams.insert(part.id, Vec::new());
        self.parts.insert(part.id, part);
        self.sort_all();
        Ok(())
    }

    // Returns the part if the part doesn't exist
    fn update_part(&mut self, part: Part) -> Result<(), Part> {
        if !self.parts.contains_key(&part.id) {
            return Err(part);
        }
        let pid = part.parent_id;
        self.parts.insert(part.id, part);
        if let Some(pid) = pid {
            let children = self
                .fams
                .get_mut(&pid)
                .expect(&format!("missing fams pid: {}", pid));
            self.sort_method.sort(&self.parts, &mut *children);
        } else {
            self.sort_projects();
        }
        Ok(())
    }

    fn delete_part(&mut self, id: i64) {
        if let Some(pid) = self.parts.remove(&id).map(|part| part.parent_id).flatten() {
            if let Some(children) = self.fams.get_mut(&pid) {
                if let Some(i) = children.iter().position(|&cid| cid == id) {
                    children.remove(i);
                }
            }
        } else if let Some(i) = self.projects.iter().position(|&pid| pid == id) {
            self.projects.remove(i);
        }
        if let Some(children) = self.fams.remove(&id) {
            children.into_iter().for_each(|cid| self.delete_part(cid));
        }
        if let Some(s) = self.filtered_ids.as_mut() {
            s.remove(&id);
        }
    }

    fn apply_sort(&mut self, method: SortMethod) {
        self.sort_method = method;
        self.sort_all();
    }

    fn sort_all(&mut self) {
        self.sort_projects();
        self.sort_children();
    }

    fn sort_projects(&mut self) {
        self.sort_method.sort(&self.parts, &mut self.projects);
    }

    fn sort_children(&mut self) {
        self.fams
            .values_mut()
            .for_each(|v| self.sort_method.sort(&self.parts, &mut *v));
    }

    fn apply_filter(&mut self, filter_method: FilterMethod) {
        self.filtered_ids = None;
        if filter_method.allows_all() {
            let filter_fn = filter_method.get_fn();
            self.filtered_ids = Some(
                self.parts
                    .iter()
                    .filter_map(|(&id, part)| filter_fn(part).then_some(id))
                    .collect(),
            );
        }
        self.filter_method = filter_method;
    }

    // Returns whether the id isn't filtered out or not
    fn id_not_filtered(&self, id: i64) -> bool {
        !self
            .filtered_ids
            .as_ref()
            .map(|s| s.contains(&id))
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMethod {
    #[default]
    Id,
    Name,
    Deadline,
    CompletedAt,
}

impl SortMethod {
    fn sort(self, parts: &PartsMap, ids: &mut [i64]) {
        match self {
            SortMethod::Id => Self::sort_by_id(ids),
            SortMethod::Name => Self::sort_by_name(parts, ids),
            SortMethod::Deadline => Self::sort_by_deadline(parts, ids),
            SortMethod::CompletedAt => Self::sort_by_completed_at(parts, ids),
        }
    }

    fn sort_by_id(ids: &mut [i64]) {
        ids.sort();
    }

    fn sort_by_name(parts: &PartsMap, ids: &mut [i64]) {
        ids.sort_by_cached_key(|id| (parts[id].name.to_lowercase(), *id));
    }

    fn sort_by_deadline(parts: &PartsMap, ids: &mut [i64]) {
        ids.sort_by_cached_key(|id| (parts[id].deadline, *id));
    }

    fn sort_by_completed_at(parts: &PartsMap, ids: &mut [i64]) {
        ids.sort_by_cached_key(|id| (parts[id].completed_at.unwrap_or(*MAX_DT), *id));
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct FilterMethod {
    // All ranges are inclusive
    // (0, MAX) = All completed
    // (None, None) = Uncompleted
    // (None, T2) = All completed before T2
    // (T1, None) = All completed after T1
    // (T1, T2) = All completed between T1 and T2
    // None = Don't filter completed at
    completed_at: Option<(Option<DateTime<Local>>, Option<DateTime<Local>>)>,
    // Same as Completed but for Deadline
    deadline: Option<(Option<DateTime<Local>>, Option<DateTime<Local>>)>,
}

// FilterFn should return true if the part should be kept
type FilterFn = Box<dyn Fn(&Part) -> bool>;

impl FilterMethod {
    fn allows_all(&self) -> bool {
        self.deadline.is_none() && self.completed_at.is_none()
    }

    fn get_fn(&self) -> FilterFn {
        let deadline_fn: FilterFn = match self.deadline {
            Some((Some(start), Some(end))) if start == *MIN_DT && end == *MAX_DT => {
                Box::new(|part| part.deadline.is_some())
            }
            Some((None, None)) => Box::new(|part| part.deadline.is_none()),
            Some((None, Some(end))) => {
                Box::new(move |part| part.deadline.map(|dt| dt <= end).unwrap_or_default())
            }
            Some((Some(start), None)) => {
                Box::new(move |part| part.deadline.map(|dt| dt >= start).unwrap_or_default())
            }
            Some((Some(start), Some(end))) => Box::new(move |part| {
                part.deadline
                    .map(|dt| dt >= start && dt <= end)
                    .unwrap_or_default()
            }),
            None => Box::new(|_| true),
        };
        let completed_at_fn: FilterFn = match self.completed_at {
            Some((Some(start), Some(end))) if start == *MIN_DT && end == *MAX_DT => {
                Box::new(|part| part.completed_at.is_some())
            }
            Some((None, None)) => Box::new(|part| part.completed_at.is_none()),
            Some((None, Some(end))) => {
                Box::new(move |part| part.completed_at.map(|dt| dt <= end).unwrap_or_default())
            }
            Some((Some(start), None)) => {
                Box::new(move |part| part.completed_at.map(|dt| dt >= start).unwrap_or_default())
            }
            Some((Some(start), Some(end))) => Box::new(move |part| {
                part.completed_at
                    .map(|dt| dt >= start && dt <= end)
                    .unwrap_or_default()
            }),
            None => Box::new(|_| true),
        };
        Box::new(move |part| deadline_fn(part) && completed_at_fn(part))
    }

    fn passes_filter(&self, part: &Part) -> bool {
        self.get_fn()(part)
    }

    fn all_completed(&self) -> bool {
        matches!(self.completed_at, Some((Some(start), Some(end))) if start == *MIN_DT && end == *MAX_DT)
    }

    fn has_deadline(&self) -> bool {
        matches!(self.deadline, Some((Some(start), Some(end))) if start == *MIN_DT && end == *MAX_DT)
    }

    fn completed_at_start_to_input(&self) -> String {
        self.completed_at
            .map(|o| o.0.map(|t| t.format(DTL_INPUT_FMT).to_string()))
            .flatten()
            .unwrap_or_default()
    }

    fn completed_at_end_to_input(&self) -> String {
        self.completed_at
            .map(|o| o.1.map(|t| t.format(DTL_INPUT_FMT).to_string()))
            .flatten()
            .unwrap_or_default()
    }

    fn deadline_start_to_input(&self) -> String {
        self.deadline
            .map(|o| o.0.map(|t| t.format(DTL_INPUT_FMT).to_string()))
            .flatten()
            .unwrap_or_default()
    }

    fn deadline_end_to_input(&self) -> String {
        self.deadline
            .map(|o| o.1.map(|t| t.format(DTL_INPUT_FMT).to_string()))
            .flatten()
            .unwrap_or_default()
    }
}

#[allow(dead_code)]
pub enum AppMsg {
    ToggleCreating,
    GetParts(PartialResult<Vec<PartsQueryParts>>),
    CreateUser(anyhow::Result<String>),
    LoginUser(anyhow::Result<String>),
    LogoutUser(anyhow::Result<bool>),
    SendCreatePart(Part),
    CreatePart(anyhow::Result<CreatePartPart>),
    Sort(SortMethod),
    Filter(FilterMethod),
    ErrorMsg(String),
    DisplayErrLogout,
}

pub struct App {
    jwt: Rc<str>,
    parts_maps: Rc<RefCell<PartsMaps>>,
    creating: bool,
    create_form_ref: NodeRef,
    filter_dd_div_ref: NodeRef,
    error_msg: String,
    send_to_app: Rc<Callback<AppMsg>>,
}

impl Component for App {
    type Message = AppMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let res = get_parts(String::new());
        ctx.link().send_future(res.map(AppMsg::GetParts));
        let link = ctx.link().clone();
        Self {
            jwt: Rc::from(""),
            parts_maps: Default::default(),
            creating: false,
            create_form_ref: NodeRef::default(),
            filter_dd_div_ref: NodeRef::default(),
            error_msg: String::new(),
            send_to_app: Rc::new(Callback::from(move |msg| {
                link.send_message(msg);
            })),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="app">
                if &*self.jwt == "" {
                    { self.render_login(ctx) }
                } else {
                    { self.render_main(ctx) }
                }
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        self.error_msg = String::new();
        // TODO: Display errors
        // TODO: Handle PartialResults
        match msg {
            AppMsg::ToggleCreating => self.creating = !self.creating,
            AppMsg::GetParts(res) => match res {
                Ok(res) => {
                    if let Some(e) = res.1 {
                        if err_is_access(e.as_ref()) {
                            if &*self.jwt != "" {
                                self.display_err_logout_alert();
                            }
                            return true;
                        }
                        self.error_msg = format!("Partial error getting projects/parts: {}", e);
                        console::log!("{}", self.error_msg);
                    }
                    *self.pm_mut() = PartsMaps::new(res.0);
                    self.jwt = Rc::from("1");
                }
                Err(e) => {
                    self.error_msg = format!("Error getting projects/parts: {}", e);
                    console::log!("{}", self.error_msg);
                }
            },
            AppMsg::CreateUser(res) => match res {
                Ok(jwt) => self.jwt = jwt.into(),
                Err(e) => {
                    self.error_msg = format!("Error creating user: {}", e);
                    console::log!("{}", self.error_msg);
                }
            },
            AppMsg::LoginUser(res) => match res {
                Ok(jwt) => {
                    self.jwt = jwt.clone().into();
                    ctx.link().send_future(get_parts(jwt).map(AppMsg::GetParts));
                }
                Err(e) => {
                    self.error_msg = format!("Error logging in: {}", e);
                    console::log!("{}", self.error_msg);
                }
            },
            AppMsg::LogoutUser(res) => {
                let window = web_sys::window().expect("no window");
                let _ = match res {
                    Ok(true) => window.alert_with_message("Successfully logged out!"),
                    Ok(false) => window.alert_with_message(
                        "Unknown error while logging out. Logging out anyway...",
                    ),
                    Err(e) => {
                        console::log!("Error logging out: {}", e);
                        window.alert_with_message(&format!(
                            "Error logging out:\n{}\nLogging out anyway...",
                            e
                        ))
                    }
                };
                self.jwt = Rc::from("");
            }
            AppMsg::SendCreatePart(part) => {
                self.creating = false;
                let res =
                    send_create_part(self.jwt.to_string(), CreatePartVars { input: part.into() });
                ctx.link().send_future(res.map(AppMsg::CreatePart));
            }
            AppMsg::CreatePart(res) => match res {
                Ok(part) => match part.try_into() {
                    Ok(part) => {
                        console::log!("New part: {:?}", part);
                        let res = self.pm_mut().add_part(part);
                        if let Err(part) = res {
                            console::log!("Part already exists: {:?}", part);
                            self.error_msg = String::from("Part already exists???");
                        }
                        console::log!("Projects: {:?}", self.pm().projects);
                    }
                    Err(e) => {
                        if err_is_access(e.as_ref()) {
                            self.display_err_logout_alert();
                            return true;
                        }
                        console::log!("Bad create part from server: {}", e);
                        self.error_msg = String::from("Internal server error");
                    }
                },
                Err(e) => {
                    self.error_msg = format!("Error creating part: {}", e);
                    console::log!("{}", self.error_msg);
                }
            },
            AppMsg::Sort(method) => self.pm_mut().apply_sort(method),
            AppMsg::Filter(method) => self.pm_mut().apply_filter(method),
            AppMsg::ErrorMsg(err_msg) => self.error_msg = err_msg,
            AppMsg::DisplayErrLogout => self.display_err_logout_alert(),
        }
        true
    }
}

impl App {
    fn render_login(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();
        let onsubmit = ctx.link().batch_callback(move |e: SubmitEvent| {
            e.prevent_default();
            let Some(submit_button) = e
                .submitter()
                .and_then(|e| e.dyn_into::<HtmlButtonElement>().ok()) else {
                    console::log!("missing or invalid submitter button");
                    return None;
                };
            let form: HtmlFormElement = e.target_unchecked_into();
            let elems = form.elements();
            let Some(email) = elems.get_with_name("email").and_then(value_from_input) else {
                console::log!("missing or invalid email input element");
                return None;
            };
            let Some(password) = elems.get_with_name("password").and_then(value_from_input) else {
                console::log!("missing or invalid password input element");
                return None;
            };
            match submit_button.name().as_str() {
                "login" => {
                    link.send_future(
                        send_login_user(LoginUserVars {
                            input: LoginUserInput { email, password },
                        })
                        .map(AppMsg::LoginUser),
                    );
                }
                "register" => {
                    link.send_future(
                        send_create_user(CreateUserVars {
                            input: CreateUserInput { email, password },
                        })
                        .map(AppMsg::CreateUser),
                    );
                }
                name => {
                    console::log!("invalid button name: {}", name);
                }
            }
            None
        });
        html! {
            <form id="login-register-div" {onsubmit}>
                <input
                    type="email"
                    name="email"
                    size="40"
                    placeholder="Email"
                    required=true
                /><br />
                <input
                    type="password"
                    name="password"
                    size="40"
                    placeholder="Password"
                    required=true
                /><br />
                <div id="logreg-button-div">
                    <button type="submit" name="login">{ "Login" }</button>
                    <button type="submit" name="register">{ "Register" }</button>
                </div>
                <p style="text-align:center; color:red;">{ &self.error_msg }</p>
            </form>
        }
    }

    fn render_main(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();
        let logout = ctx.link().batch_callback(move |_| {
            link.send_future(send_logout_user().map(AppMsg::LogoutUser));
            None
        });
        html! {
            <div id="main-div">
                <button id="logout-button" onclick={logout}>{ "Logout" }</button>
                <h1 style="text-align:center">{ "Let's Get Productive!" }</h1>
                <p id="err-msg-p">{self.error_msg.as_str()}</p>
                { self.render_controls(ctx) }
                { self.render_projects(ctx) }
            </div>
        }
    }

    // TODO: Rename?
    fn render_controls(&self, ctx: &Context<Self>) -> Html {
        let change_sort = ctx.link().batch_callback(|e: Event| {
            let Some(select) = e.target_dyn_into::<HtmlSelectElement>() else {
                console::log!("missing or invalid sort select element");
                return None;
            };
            match select.value().as_str() {
                "id" => Some(AppMsg::Sort(SortMethod::Id)),
                "name" => Some(AppMsg::Sort(SortMethod::Name)),
                "deadline" => Some(AppMsg::Sort(SortMethod::Deadline)),
                "completedAt" => Some(AppMsg::Sort(SortMethod::CompletedAt)),
                val => {
                    console::log!("invalid select value: {}", val);
                    None
                }
            }
        });

        let dropdown = self.filter_dd_div_ref.clone();
        let show_filter = ctx.link().batch_callback(move |_| {
            let elem = dropdown.cast::<HtmlElement>().unwrap();
            elem.set_hidden(!elem.hidden());
            None
        });

        let toggle_creating = ctx.link().callback(|_| AppMsg::ToggleCreating);
        html! {
            <div id="top-controls-div">
                <button onclick={toggle_creating}>{ "New Project" }</button>
                <input type="text" placeholder="Search" />

                <label for="sort">{ "Sort" }</label>
                <select onchange={change_sort}>
                    <option value="id" selected=true>{ "Added" }</option>
                    <option value="name">{ "Name" }</option>
                    <option value="deadline">{ "Deadline" }</option>
                    <option value="completedAt">{ "Completed At" }</option>
                </select>
                <div style="float:left; overflow:hidden">
                    <button onclick={show_filter}>{ "Filter" }</button>
                    { self.render_filter_popup(ctx) }
                </div>
            </div>
        }
    }

    fn render_projects(&self, ctx: &Context<Self>) -> Html {
        let parts_maps = self.pm();
        html! {
            if parts_maps.parts.len() != 0 {
                <div id="projects-div">
                    <ul class="parts-list">
                        { self.render_create_project(ctx) }
                        {
                            parts_maps.projects.iter()
                                .map(|&id| {
                                    html! {
                                        <PartComponent
                                            key={id}
                                            id={id}
                                            parts_maps={Rc::clone(&self.parts_maps)}
                                            jwt={Rc::clone(&self.jwt)}
                                            send_to_app={Rc::clone(&self.send_to_app)}
                                        />
                                    }
                                }).collect::<Html>()
                        }
                    </ul>
                </div>
            } else {
                <p>{ "No projects" }</p>
                { self.render_create_project(ctx) }
            }
        }
    }

    #[allow(unused_variables)]
    fn render_filter_popup(&self, ctx: &Context<Self>) -> Html {
        let filter_method = self.parts_maps.borrow().filter_method;
        let deadline_start = filter_method.deadline_start_to_input();
        let deadline_end = filter_method.deadline_end_to_input();
        let completed_at_start = filter_method.completed_at_start_to_input();
        let completed_at_end = filter_method.completed_at_end_to_input();
        let all_completed = filter_method.all_completed();
        let has_deadline = filter_method.has_deadline();
        html! {
            <div
                id="filter-dropdown-div"
                ref={self.filter_dd_div_ref.clone()}
                hidden=true
            >
                <form>
                    <h5><u>{ "Deadline" }</u></h5>
                    <label for="deadline-start-input">{ "Start: " }</label>
                    <input
                        type="datetime-local"
                        name="deadline-start-input"
                        value={deadline_start}
                    />
                    <input
                        type="button"
                        name="clear-deadline-start"
                        value="X"
                    />
                    <br />

                    <label for="deadline-end-input">{ "End: " }</label>
                    <input
                        type="datetime-local"
                        name="deadline-end-input"
                        value={deadline_end}
                    />
                    <input
                        type="button"
                        name="clear-deadline-end"
                        value="X"
                    />
                    <br />

                    <h5><u>{ "Completed At" }</u></h5>
                    <label for="comp-at-start-input">{ "Start: " }</label>
                    <input
                        type="datetime-local"
                        name="comp-at-start-input"
                        value={completed_at_start}
                    />
                    <input
                        type="button"
                        name="clear-comp-at-start"
                        value="X"
                    />
                    <br />

                    <label for="comp-at-end-input">{ "End: " }</label>
                    <input
                        type="datetime-local"
                        name="comp-at-end-input"
                        value={completed_at_end}
                    />
                    <input
                        type="button"
                        name="clear-comp-at-end"
                        value="X"
                    />
                    <br />

                    <br />
                    <input type="button" value="Reset" /><br />

                    <input type="button" value="Apply" />
                    <input type="button" value="Cancel" />
                </form>
            </div>
        }
    }

    fn render_create_project(&self, ctx: &Context<Self>) -> Html {
        let toggle_creating = ctx.link().callback(|_| AppMsg::ToggleCreating);

        let create_form_ref = self.create_form_ref.clone();
        let send_create = ctx.link().batch_callback(move |_| {
            let form = create_form_ref.cast::<HtmlFormElement>().unwrap();
            let elems = form.elements();
            let mut part = Part::default();

            if let Some(name) = elems.get_with_name("part-name").and_then(value_from_input) {
                if name.trim() == "" {
                    // TODO: Display error
                    return None;
                }
                part.name = name;
            } else {
                console::log!("missing or invalid name input element");
                return None;
            };
            if let Some(desc) = elems.get_with_name("part-desc").and_then(value_from_area) {
                if desc.trim() != "" {
                    part.description = Some(desc);
                }
            } else {
                console::log!("missing or invalid description text area element");
                return None;
            };
            if let Some(dtl) = elems
                .get_with_name("part-deadline")
                .and_then(dtl_from_input)
            {
                // TODO: Do better
                if dtl != DateTime::<Local>::default() {
                    part.deadline = Some(dtl);
                }
            } else {
                console::log!("missing or invalid deadline input element");
                return None;
            };
            if let Some(dtl) = elems.get_with_name("part-comp-at").and_then(dtl_from_input) {
                // TODO: Do better
                if dtl != DateTime::<Local>::default() {
                    part.completed_at = Some(dtl);
                }
            } else {
                console::log!("missing or invalid comp-at input element");
                return None;
            };
            Some(AppMsg::SendCreatePart(part))
        });
        html! {
            <>
            if self.creating {
                // TODO: Make more reusable (see other instances of this form
                <button onclick={send_create} style="margin:10px">{ "Create" }</button>
                <button onclick={toggle_creating}>{ "Cancel" }</button>
                <form ref={self.create_form_ref.clone()}>
                    <label for="part-name"><u>{ "Name" }</u>{ ": " }</label>
                    <input
                        name="part-name"
                        type="text"
                        placeholder="Name"
                        required=true
                    /><br />

                    <label for="part-desc"><u>{ "Description" }</u>{ ":" }</label><br />
                    <textarea name="part-desc" placeholder="Description">
                    </textarea>
                    <br />

                    <label for="part-deadline"><u>{ "Deadline" }</u>{ ": " }</label>
                    <input
                        name="part-deadline"
                        type="datetime-local"
                    /><br />

                    <label for="part-comp-at"><u>{ "Completed At" }</u>{ ": " }</label>
                    <input
                        name="part-comp-at"
                        type="datetime-local"
                    /><br />
                </form>
            }
            </>
        }
    }

    // Displayed if unexpected Access denied is returned
    fn display_err_logout_alert(&mut self) {
        self.jwt = Rc::from("");
        let _ = web_sys::window()
            .expect("no window")
            .alert_with_message("Unexpectedly logged out");
    }

    fn pm(&self) -> Ref<'_, PartsMaps> {
        self.parts_maps.borrow()
    }

    fn pm_mut(&self) -> RefMut<'_, PartsMaps> {
        self.parts_maps.borrow_mut()
    }
}

#[derive(Properties, PartialEq)]
pub struct PartProps {
    id: i64,
    parts_maps: Rc<RefCell<PartsMaps>>,
    jwt: Rc<str>,
    send_to_app: Rc<Callback<AppMsg>>,
}

pub enum PartMsg {
    ToggleCreating,
    ToggleEditing,
    ToggleHide,
    CreatePart(anyhow::Result<CreatePartPart>),
    UpdatePart(anyhow::Result<UpdatePartPart>),
    DeletePart(anyhow::Result<String>),
}

pub struct PartComponent {
    creating: bool,
    editing: bool,
    child_list_ref: NodeRef,
    details_div_ref: NodeRef,
    create_form_ref: NodeRef,
    updates_form_ref: NodeRef,
    hide_children: bool,
}

impl Component for PartComponent {
    type Message = PartMsg;
    type Properties = PartProps;

    fn create(_: &Context<Self>) -> Self {
        Self {
            creating: false,
            editing: false,
            child_list_ref: NodeRef::default(),
            details_div_ref: NodeRef::default(),
            create_form_ref: NodeRef::default(),
            updates_form_ref: NodeRef::default(),
            hide_children: true,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let id = ctx.props().id;
        let parts_maps = ctx.props().parts_maps.borrow();
        let part = &parts_maps.parts[&id];
        let children = &parts_maps.fams[&id];

        let child_list_ref = self.child_list_ref.clone();
        let show_children = ctx.link().batch_callback(move |_| {
            if false {
                let elem = child_list_ref.cast::<HtmlElement>().unwrap();
                elem.set_hidden(!elem.hidden());
            }
            //None
            Some(PartMsg::ToggleHide)
        });

        let details_div_ref = self.details_div_ref.clone();
        let show_details = ctx.link().batch_callback(move |_| {
            let elem = details_div_ref.cast::<HtmlElement>().unwrap();
            elem.set_hidden(!elem.hidden());
            None
        });

        let toggle_creating = ctx.link().callback(|_| PartMsg::ToggleCreating);
        html! {
            <li class="part">
                <span>{ &part.name }</span>
                <button onclick={show_details}>{ "Details" }</button>
                <button onclick={toggle_creating}>{ "New Part" }</button>
                if children.len() != 0 {
                    <button onclick={show_children}>{ 
                        if self.hide_children { "Show Children" } else { "Hide Children" }
                    }</button>
                }

                { self.render_details(ctx, part) }

                <ul class="parts-list" ref={self.child_list_ref.clone()} hidden=false>
                { self.render_create_part(ctx) }
                if !self.hide_children {
                    {
                        children.into_iter().filter_map(|&child_id| {
                            parts_maps.id_not_filtered(child_id).then(|| html! {
                                <PartComponent
                                    key={child_id}
                                    id={child_id}
                                    parts_maps={Rc::clone(&ctx.props().parts_maps)}
                                    jwt={Rc::clone(&ctx.props().jwt)}
                                    send_to_app={Rc::clone(&ctx.props().send_to_app)}
                                />
                            })
                        }).collect::<Html>()
                    }
                }
                </ul>
                /*
                if children.len() != 0 {
                } else {
                    { self.render_create_part(ctx) }
                }
                */
            </li>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        // TODO: Show logout error on err_is_access
        match msg {
            PartMsg::ToggleCreating => self.creating = !self.creating,
            PartMsg::ToggleEditing => self.editing = !self.editing,
            PartMsg::ToggleHide => self.hide_children = !self.hide_children,
            PartMsg::CreatePart(res) => match res {
                Ok(part) => match part.try_into() {
                    Ok(part) => {
                        let res = ctx.props().parts_maps.borrow_mut().add_part(part);
                        if let Err(part) = res {
                            console::log!("Part already exists: {:?}", part);
                            ctx.props()
                                .send_to_app
                                .emit(AppMsg::ErrorMsg(String::from("Part already exists???")));
                        }
                    }
                    Err(e) => {
                        if err_is_access(e.as_ref()) {
                            ctx.props().send_to_app.emit(AppMsg::DisplayErrLogout);
                            return true;
                        }
                        console::log!("Bad create part from server: {}", e);
                        ctx.props()
                            .send_to_app
                            .emit(AppMsg::ErrorMsg(String::from("Internal Server Error")));
                    }
                },
                Err(e) => {
                    let error_msg = format!("Error creating part: {}", e);
                    console::log!("{}", error_msg);
                    ctx.props().send_to_app.emit(AppMsg::ErrorMsg(error_msg));
                }
            },
            PartMsg::UpdatePart(res) => match res {
                Ok(part) => match part.try_into() {
                    Ok(part) => {
                        if let Err(part) = ctx.props().parts_maps.borrow_mut().update_part(part) {
                            console::log!("Part doesn't exist: {:?}", part);
                            ctx.props()
                                .send_to_app
                                .emit(AppMsg::ErrorMsg(String::from("Part doesn't exist???")));
                        }
                    }
                    Err(e) => console::log!("bad update part from server: {}", e),
                },
                Err(e) => {
                    console::log!("error updating part: {}", e);
                    ctx.props()
                        .send_to_app
                        .emit(AppMsg::ErrorMsg(e.to_string()));
                }
            },
            PartMsg::DeletePart(res) => match res {
                Ok(_) => {
                    // TODO: Possibly use returned id (string)
                    ctx.props()
                        .parts_maps
                        .borrow_mut()
                        .delete_part(ctx.props().id);
                }
                Err(e) => {
                    console::log!("error deleting part: {}", e);
                    ctx.props()
                        .send_to_app
                        .emit(AppMsg::ErrorMsg(e.to_string()));
                }
            },
        }
        true
    }
}

impl PartComponent {
    fn render_details(&self, ctx: &Context<Self>, part: &Part) -> Html {
        let toggle_editing = ctx.link().callback(|_| PartMsg::ToggleEditing);

        let id = part.id;
        let parts_maps = Rc::clone(&ctx.props().parts_maps);
        let jwt = Rc::clone(&ctx.props().jwt);
        let updates_form_ref = self.updates_form_ref.clone();
        let link = ctx.link().clone();
        let send_updates = ctx.link().batch_callback(move |_| {
            let form = updates_form_ref.cast::<HtmlFormElement>().unwrap();
            let elems = form.elements();
            let mut changes = HashMap::new();
            let part = &parts_maps.borrow().parts[&id];

            if let Some(name) = elems.get_with_name("part-name").and_then(value_from_input) {
                if name != part.name {
                    changes.insert(String::from("name"), name);
                }
            } else {
                console::log!("missing or invalid name input element");
                return None;
            };
            if let Some(desc) = elems.get_with_name("part-desc").and_then(value_from_area) {
                if part
                    .description
                    .as_ref()
                    .map(|d| d != &desc)
                    .unwrap_or(desc.len() != 0)
                {
                    changes.insert(String::from("description"), desc);
                }
            } else {
                console::log!("missing or invalid description text area element");
                return None;
            };
            if let Some(dtl) = elems
                .get_with_name("part-deadline")
                .and_then(dtl_from_input)
            {
                if part
                    .deadline
                    .map(|d| d != dtl)
                    .unwrap_or(dtl != DateTime::<Local>::default())
                {
                    changes.insert(String::from("deadline"), dtl.timestamp().to_string());
                }
            } else {
                console::log!("missing or invalid deadline input element");
                return None;
            };
            if let Some(dtl) = elems.get_with_name("part-comp-at").and_then(dtl_from_input) {
                if part
                    .completed_at
                    .map(|d| d != dtl)
                    .unwrap_or(dtl != DateTime::<Local>::default())
                {
                    changes.insert(String::from("completed_at"), dtl.timestamp().to_string());
                }
            } else {
                console::log!("missing or invalid comp-at input element");
                return None;
            };
            if changes.len() != 0 {
                let res = send_update_part(
                    jwt.to_string(),
                    UpdatePartVars {
                        id: id.to_string(),
                        changes,
                    },
                );
                link.send_future(res.map(PartMsg::UpdatePart));
            }
            Some(PartMsg::ToggleEditing)
        });

        let id = part.id;
        let parts_maps = Rc::clone(&ctx.props().parts_maps);
        let jwt = Rc::clone(&ctx.props().jwt);
        let link = ctx.link().clone();
        let confirm_delete = ctx.link().batch_callback(move |_| {
            let res = web_sys::window()
                .expect("no window")
                .confirm_with_message(&format!(
                    "Delete the following project/part and ALL its children?\n{}",
                    parts_maps.borrow().parts[&id].name,
                ));
            match res {
                Ok(true) => {
                    let res =
                        send_delete_part(jwt.to_string(), DeletePartVars { id: id.to_string() });
                    link.send_future(res.map(PartMsg::DeletePart));
                    return Some(PartMsg::ToggleEditing);
                }
                Ok(false) => (),
                Err(e) => console::log!("error confirming delete: {:?}", e),
            }
            None
        });
        html! {
            <div ref={self.details_div_ref.clone()} hidden=true>
                if !self.editing {
                    <button onclick={toggle_editing}>{ "Edit" }</button>
                    <p>
                        <u>{ "Name" }</u>{ format!(": {}", part.name) }<br />

                        <u>{ "Description" }</u>{ ":" }<br />
                        if let Some(desc) = part.description.as_ref() {
                            { desc }<br />
                        }

                        <u>{ "Deadline" }</u>
                        { format!(
                            ": {}",
                            part.deadline_to_string(),
                        )}<br />

                        <u>{ "Completed At" }</u>
                        { format!(
                            ": {}",
                            part.completed_at_to_string(),
                        )}<br />
                    </p>
                } else {
                    <button onclick={send_updates} style="margin: 10px">{ "Save" }</button>
                    <button onclick={toggle_editing}>{ "Cancel" }</button>
                    <button class="delete-button" onclick={confirm_delete}>{ "Delete" }</button>
                    <form ref={self.updates_form_ref.clone()}>
                        <label for="part-name"><u>{ "Name" }</u>{ ": " }</label>
                        <input
                            name="part-name"
                            type="text"
                            placeholder="Name"
                            value={part.name.clone()}
                            required=true
                        /><br />

                        <label for="part-desc"><u>{ "Description" }</u>{ ":" }</label><br />
                        <textarea name="part-desc" placeholder="Description">
                            { part.description.clone().unwrap_or_default() }
                        </textarea>
                        <br />

                        <label for="part-deadline"><u>{ "Deadline" }</u>{ ": " }</label>
                        <input
                            name="part-deadline"
                            type="datetime-local"
                            value={part.deadline_to_input()}
                        /><br />

                        <label for="part-comp-at"><u>{ "Completed At" }</u>{ ": " }</label>
                        <input
                            name="part-comp-at"
                            type="datetime-local"
                            value={part.completed_at_to_input()}
                        /><br />
                    </form>
                }
            </div>
        }
    }

    fn render_create_part(&self, ctx: &Context<Self>) -> Html {
        let id = ctx.props().id;
        let send_to_app = Rc::clone(&ctx.props().send_to_app);
        let link = ctx.link().clone();
        let jwt = Rc::clone(&ctx.props().jwt);
        let create_form_ref = self.create_form_ref.clone();
        let send_create = ctx.link().batch_callback(move |_| {
            let form = create_form_ref.cast::<HtmlFormElement>().unwrap();
            let elems = form.elements();
            let mut part = Part::default();

            if let Some(name) = elems.get_with_name("part-name").and_then(value_from_input) {
                if name.trim() == "" {
                    // TODO: Display error
                    send_to_app.emit(AppMsg::ErrorMsg(String::from("Must provide name")));
                    return None;
                }
                part.name = name;
            } else {
                console::log!("missing or invalid name input element");
                send_to_app.emit(AppMsg::ErrorMsg(String::from("Bad form")));
                return None;
            };
            if let Some(desc) = elems.get_with_name("part-desc").and_then(value_from_area) {
                if desc.trim() != "" {
                    part.description = Some(desc);
                }
            } else {
                console::log!("missing or invalid description text area element");
                send_to_app.emit(AppMsg::ErrorMsg(String::from("Bad form")));
                return None;
            };
            if let Some(dtl) = elems
                .get_with_name("part-deadline")
                .and_then(dtl_from_input)
            {
                // TODO: Do better
                if dtl != DateTime::<Local>::default() {
                    part.deadline = Some(dtl);
                }
            } else {
                console::log!("missing or invalid deadline input element");
                send_to_app.emit(AppMsg::ErrorMsg(String::from("Bad form")));
                return None;
            };
            if let Some(dtl) = elems.get_with_name("part-comp-at").and_then(dtl_from_input) {
                // TODO: Do better
                if dtl != DateTime::<Local>::default() {
                    part.completed_at = Some(dtl);
                }
            } else {
                console::log!("missing or invalid comp-at input element");
                send_to_app.emit(AppMsg::ErrorMsg(String::from("Bad form")));
                return None;
            };
            part.parent_id = Some(id);
            //send_to_app.emit(AppMsg::SendCreatePart(part));
            let res = send_create_part(jwt.to_string(), CreatePartVars { input: part.into() });
            link.send_future(res.map(PartMsg::CreatePart));
            Some(PartMsg::ToggleCreating)
        });

        let toggle_creating = ctx.link().callback(|_| PartMsg::ToggleCreating);
        html! {
            <>
            if self.creating {
                // TODO: Make more reusable (see other instances of this form)
                <li>
                <button onclick={send_create} style="margin:10px">{ "Create" }</button>
                <button onclick={toggle_creating}>{ "Cancel" }</button>
                <form ref={self.create_form_ref.clone()}>
                    <label for="part-name"><u>{ "Name" }</u>{ ": " }</label>
                    <input
                        name="part-name"
                        type="text"
                        placeholder="Name"
                        required=true
                    /><br />

                    <label for="part-desc"><u>{ "Description" }</u>{ ":" }</label><br />
                    <textarea name="part-desc" placeholder="Description">
                    </textarea>
                    <br />

                    <label for="part-deadline"><u>{ "Deadline" }</u>{ ": " }</label>
                    <input
                        name="part-deadline"
                        type="datetime-local"
                    /><br />

                    <label for="part-comp-at"><u>{ "Completed At" }</u>{ ": " }</label>
                    <input
                        name="part-comp-at"
                        type="datetime-local"
                    /><br />
                </form>
                </li>
            }
            </>
        }
    }
}

fn value_from_input(elem: Element) -> Option<String> {
    elem.dyn_into::<HtmlInputElement>()
        .ok()
        .map(|input| input.value())
}

fn value_from_area(elem: Element) -> Option<String> {
    elem.dyn_into::<HtmlTextAreaElement>()
        .ok()
        .map(|area| area.value())
}

// TODO: Do better
fn dtl_from_input(elem: Element) -> Option<DateTime<Local>> {
    value_from_input(elem).map(|val| Part::dtl_from_input(val.as_str()).unwrap_or_default())
}
