use self::{
    create_part_mutation::{
        CreatePartInput, CreatePartMutationCreatePart, Variables as CreatePartVars,
    },
    create_user_mutation::Variables as CreateUserVars,
    delete_part_mutation::Variables as DeletePartVars,
    login_user_mutation::Variables as LoginUserVars,
    logout_user_mutation::Variables as LogoutUserVars,
    parts_query::{PartsQueryParts, Variables as PartsVars},
    update_part_mutation::{UpdatePartMutationUpdatePart, Variables as UpdatePartVars},
};
use crate::console;
use chrono::prelude::*;
use graphql_client::{
    reqwest::{post_graphql, post_graphql_req},
    reqwest_crate::Client,
    GraphQLQuery,
};

lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
    static ref QUERY_URL: String = web_sys::Url::new_with_base(
        "/query",
        web_sys::window()
            .expect("no window")
            .location()
            .href()
            .expect("error getting location href").as_str(),
    )
        .expect("error creating query url")
        .href();
    pub static ref TZ: FixedOffset = Local::now().offset().fix();
}

type Id = String;

// TODO: Change to be better representation?
type Map = std::collections::HashMap<String, String>;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/queries.graphql",
    response_derives = "Debug, PartialEq",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct PartsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct CreateUserMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct LoginUserMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct LogoutUserMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct CreatePartMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct UpdatePartMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    normalization = "rust"
)]
pub struct DeletePartMutation;

// TODO: Handle errors (even when data is also returned)
pub async fn get_parts(jwt: String) -> PartialResult<Vec<PartsQueryParts>> {
    match post_graphql_req::<PartsQuery>(
        CLIENT
            .post(QUERY_URL.as_str())
            .header("Authorization", format!("bearer {}", jwt)),
        PartsVars {},
    )
    .await
    {
        Ok(resp) => {
            let err = resp.errors.map(|errs| {
                errs.into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .map_val(anyhow::Error::msg)
            });
            let data = if let Some(resp_data) = resp.data {
                resp_data.parts
            } else {
                Vec::new()
            };
            Ok(PartialOk(data, err))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn send_login_user(vars: LoginUserVars) -> anyhow::Result<String> {
    match post_graphql::<LoginUserMutation, _>(&CLIENT, QUERY_URL.as_str(), vars).await {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                // TODO: Combine errors in a better way?
                Err(anyhow::anyhow!(
                    "{}",
                    errors
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            } else if let Some(jwt) = resp.data {
                Ok(jwt.login_user)
            } else {
                console::log!("no data or error received");
                Err(anyhow::anyhow!("Internal server error"))
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn send_create_user(vars: CreateUserVars) -> anyhow::Result<String> {
    match post_graphql::<CreateUserMutation, _>(&CLIENT, QUERY_URL.as_str(), vars).await {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                // TODO: Combine errors in a better way?
                Err(anyhow::anyhow!(
                    "{}",
                    errors
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            } else if let Some(jwt) = resp.data {
                Ok(jwt.create_user)
            } else {
                console::log!("no data or error received");
                Err(anyhow::anyhow!("Internal server error"))
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn send_logout_user() -> anyhow::Result<bool> {
    match post_graphql::<LogoutUserMutation, _>(&CLIENT, QUERY_URL.as_str(), LogoutUserVars {})
        .await
    {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                Err(anyhow::anyhow!(
                    "{}",
                    errors
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            } else if let Some(b) = resp.data {
                Ok(b.logout_user)
            } else {
                console::log!("no data or error received on logout");
                Err(anyhow::anyhow!("Internal server error"))
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn send_create_part(
    jwt: String,
    vars: CreatePartVars,
) -> anyhow::Result<CreatePartMutationCreatePart> {
    match post_graphql_req::<CreatePartMutation>(
        CLIENT
            .post(QUERY_URL.as_str())
            .header("Authorization", format!("bearer {}", jwt)),
        vars,
    )
    .await
    {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                // TODO: Combine errors in a better way?
                Err(anyhow::anyhow!(
                    "{}",
                    errors
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            } else if let Some(resp_data) = resp.data {
                Ok(resp_data.create_part)
            } else {
                console::log!("no data or error received");
                Err(anyhow::anyhow!("Internal server error"))
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn send_update_part(
    jwt: String,
    vars: UpdatePartVars,
) -> anyhow::Result<UpdatePartMutationUpdatePart> {
    match post_graphql_req::<UpdatePartMutation>(
        CLIENT
            .post(QUERY_URL.as_str())
            .header("Authorization", format!("bearer {}", jwt)),
        vars,
    )
    .await
    {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                // TODO: Combine errors in a better way?
                Err(anyhow::anyhow!(
                    "{}",
                    errors
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            } else if let Some(resp_data) = resp.data {
                Ok(resp_data.update_part)
            } else {
                console::log!("no data or error received");
                Err(anyhow::anyhow!("Internal server error"))
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn send_delete_part(jwt: String, vars: DeletePartVars) -> anyhow::Result<String> {
    match post_graphql_req::<DeletePartMutation>(
        CLIENT
            .post(QUERY_URL.as_str())
            .header("Authorization", format!("bearer {}", jwt)),
        vars,
    )
    .await
    {
        Ok(resp) => {
            if let Some(errors) = resp.errors {
                // TODO: Combine errors in a better way?
                Err(anyhow::anyhow!(
                    "{}",
                    errors
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            } else if let Some(resp_data) = resp.data {
                Ok(resp_data.delete_part)
            } else {
                console::log!("no data or error received");
                Err(anyhow::anyhow!("Internal server error"))
            }
        }
        Err(e) => Err(e.into()),
    }
}

const DTL_FMT: &str = "%H:%M %b %d, %Y";
pub const DTL_INPUT_FMT: &str = "%Y-%m-%dT%H:%M";

type NDT = NaiveDateTime;

trait MapValue<V> {
    fn map_val(self, f: impl Fn(Self) -> V) -> V
    where
        Self: Sized,
    {
        f(self)
    }
}

impl<T, V> MapValue<V> for T {}

pub struct PartialOk<T>(pub T, pub Option<anyhow::Error>);

impl<T: std::fmt::Debug> std::fmt::Debug for PartialOk<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "({:?}, {:?})", self.0, self.1)
    }
}

pub type PartialResult<T> = anyhow::Result<PartialOk<T>>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Part {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub deadline: Option<DateTime<Local>>,
    pub completed_at: Option<DateTime<Local>>,
    pub parent_id: Option<i64>,
}

impl Part {
    pub fn deadline_to_input(&self) -> String {
        self.deadline
            .map(|dt| dt.format(DTL_INPUT_FMT).to_string())
            .unwrap_or_default()
    }

    pub fn completed_at_to_input(&self) -> String {
        self.completed_at
            .map(|dt| dt.format(DTL_INPUT_FMT).to_string())
            .unwrap_or_default()
    }

    pub fn deadline_to_string(&self) -> String {
        self.deadline
            .map(|dt| dt.format(DTL_FMT).to_string())
            .unwrap_or_default()
    }

    pub fn completed_at_to_string(&self) -> String {
        self.completed_at
            .map(|dt| dt.format(DTL_FMT).to_string())
            .unwrap_or_default()
    }

    pub fn dtl_from_input(s: &str) -> Option<DateTime<Local>> {
        //TZ.datetime_from_str(s, DTL_INPUT_FMT).ok()
        Local.datetime_from_str(s, DTL_INPUT_FMT).ok()
    }
}

/*
impl From<PartsQueryParts> for Part {
    fn from(part: PartsQueryParts) -> Self {
        Self {
            id: part.id.parse().unwrap_or(0),
            name: part.name,
            description: part.description,
            deadline: part
                .deadline
                .map(|t_str| t_str.parse::<i64>().ok())
                .flatten()
                .map(|t| NDT::from_timestamp_opt(t, 0))
                .flatten()
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            completed_at: part
                .completed_at
                .map(|t_str| t_str.parse::<i64>().ok())
                .flatten()
                .map(|t| NDT::from_timestamp_opt(t, 0))
                .flatten()
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            parent_id: part.parent_id.map(|pid| pid.parse().ok()).flatten(),
        }
    }
}
*/

impl TryFrom<PartsQueryParts> for Part {
    type Error = anyhow::Error;

    fn try_from(part: PartsQueryParts) -> Result<Self, Self::Error> {
        Ok(Self {
            id: part.id.parse()?,
            name: part.name,
            description: part.description,
            deadline: part
                .deadline
                .map(|t_str| t_str.parse::<i64>())
                .transpose()?
                .map(|t| NDT::from_timestamp_opt(t, 0).ok_or(anyhow::anyhow!("deadline: {}", t)))
                .transpose()?
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            completed_at: part
                .completed_at
                .map(|t_str| t_str.parse::<i64>())
                .transpose()?
                .map(|t| {
                    NDT::from_timestamp_opt(t, 0).ok_or(anyhow::anyhow!("completed_at: {}", t))
                })
                .transpose()?
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            parent_id: part.parent_id.map(|pid| pid.parse()).transpose()?,
        })
    }
}

impl TryFrom<CreatePartMutationCreatePart> for Part {
    type Error = anyhow::Error;

    fn try_from(part: CreatePartMutationCreatePart) -> Result<Self, Self::Error> {
        Ok(Self {
            id: part.id.parse()?,
            name: part.name,
            description: part.description,
            deadline: part
                .deadline
                .map(|t_str| t_str.parse::<i64>())
                .transpose()?
                .map(|t| NDT::from_timestamp_opt(t, 0).ok_or(anyhow::anyhow!("deadline: {}", t)))
                .transpose()?
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            completed_at: part
                .completed_at
                .map(|t_str| t_str.parse::<i64>())
                .transpose()?
                .map(|t| {
                    NDT::from_timestamp_opt(t, 0).ok_or(anyhow::anyhow!("completed_at: {}", t))
                })
                .transpose()?
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            parent_id: part.parent_id.map(|pid| pid.parse()).transpose()?,
        })
    }
}

impl TryFrom<UpdatePartMutationUpdatePart> for Part {
    type Error = anyhow::Error;

    fn try_from(part: UpdatePartMutationUpdatePart) -> Result<Self, Self::Error> {
        Ok(Self {
            id: part.id.parse()?,
            name: part.name,
            description: part.description,
            deadline: part
                .deadline
                .map(|t_str| t_str.parse::<i64>())
                .transpose()?
                .map(|t| NDT::from_timestamp_opt(t, 0).ok_or(anyhow::anyhow!("deadline: {}", t)))
                .transpose()?
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            completed_at: part
                .completed_at
                .map(|t_str| t_str.parse::<i64>())
                .transpose()?
                .map(|t| {
                    NDT::from_timestamp_opt(t, 0).ok_or(anyhow::anyhow!("completed_at: {}", t))
                })
                .transpose()?
                .map(|ndt| DateTime::<Local>::from_utc(ndt, *TZ)),
            parent_id: part.parent_id.map(|pid| pid.parse()).transpose()?,
        })
    }
}

impl Into<CreatePartInput> for Part {
    fn into(self) -> CreatePartInput {
        CreatePartInput {
            name: self.name,
            description: self.description,
            deadline: self.deadline.map(|dt| dt.timestamp().to_string()),
            completed_at: self.completed_at.map(|dt| dt.timestamp().to_string()),
            parent_id: self.parent_id.map(|id| id.to_string()),
        }
    }
}

pub fn err_is_access(err: &dyn std::error::Error) -> bool {
    err.to_string().ends_with("Access denied")
}
