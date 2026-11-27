//! An MCP server over one living-docs bundle: the `write`, `read`, `manage`
//! and `check` tools, each calling the same `living-docs-core` entry point as
//! its CLI verb, plus the embedded skills through the MCP skills extension
//! and as `skill://` resources, plus the records in force as `record://`
//! resources whose changes reach `subscriptions/listen` listeners.

mod args;
mod events;
mod instructions;
mod records;
mod skills;
mod tasks;
mod tools;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CancelTaskParams, GetSkillRequestParams,
    GetSkillResult, GetTaskParams, GetTaskResult, Implementation, ListResourcesResult,
    ListSkillsResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ServerCapabilities, ServerConfig, SubscriptionFilter, Tool,
};
use rmcp::service::{RequestContext, SubscriptionContext};
use rmcp::transport::stdio;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use std::path::{Path, PathBuf};

const TOOL_ORDER: [&str; 4] = ["write", "read", "manage", "check"];

/// The handler for one bundle; no tool takes a path to another one.
#[derive(Clone)]
pub struct BundleServer {
    docs_dir: PathBuf,
    events: events::Events,
    check_tasks: tasks::CheckTasks,
    tool_router: ToolRouter<Self>,
}

impl BundleServer {
    pub fn new(docs_dir: &Path) -> Self {
        Self::with_check_task_threshold(docs_dir, tasks::CHECK_TASK_THRESHOLD)
    }

    /// The server for `docs_dir` whose `check` runs as a task from `threshold`
    /// records in force.
    pub fn with_check_task_threshold(docs_dir: &Path, threshold: usize) -> Self {
        Self {
            docs_dir: docs_dir.to_path_buf(),
            events: events::Events::new(docs_dir),
            check_tasks: tasks::CheckTasks::new(threshold),
            tool_router: Self::tool_router(),
        }
    }

    fn ordered_tools(&self) -> Vec<Tool> {
        let mut tools = self.tool_router.list_all();
        tools.sort_by_key(|tool| TOOL_ORDER.iter().position(|name| *name == tool.name));
        tools
    }
}

impl ServerHandler for BundleServer {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_resources_list_changed()
            .enable_resources_subscribe()
            .enable_skills()
            .enable_tasks()
            .build();
        ServerConfig::new(capabilities)
            .with_server_info(Implementation::new(
                "living-docs-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(instructions::instructions(&self.docs_dir))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.ordered_tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let declared = context
            .client_capabilities()
            .is_some_and(|capabilities| capabilities.supports_tasks());
        if let Some(args) = self.check_tasks.wanted(&self.docs_dir, &request, declared) {
            let created = self.check_tasks.spawn(self.clone(), args);
            return Ok(CallToolResponse::Task(created));
        }
        let may_write = matches!(request.name.as_ref(), "write" | "manage");
        let call = ToolCallContext::new(self, request, context);
        let result = self.tool_router.call(call).await;
        if may_write {
            self.events.wrote();
        }
        result
    }

    async fn get_task(
        &self,
        request: GetTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetTaskResult, ErrorData> {
        self.check_tasks.get(&request.task_id)
    }

    async fn cancel_task(
        &self,
        request: CancelTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        self.check_tasks.cancel(&request.task_id)
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(requested.clone())
    }

    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        self.events.attend(context).await;
        Ok(())
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_router.get(name).cloned()
    }

    async fn list_skills(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListSkillsResult, ErrorData> {
        skills::skills().map(ListSkillsResult::with_all_items)
    }

    async fn get_skill(
        &self,
        request: GetSkillRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetSkillResult, ErrorData> {
        skills::skill(&request.uri).map(GetSkillResult::new)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut resources = skills::resources();
        resources.extend(records::resources(&self.docs_dir));
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let read = if request.uri.starts_with(records::SCHEME) {
            records::read(&self.docs_dir, &request.uri)
        } else {
            skills::read(&request.uri)
        };
        read.map(ReadResourceResponse::Complete)
    }
}

/// Serves the bundle at `docs_dir` to the MCP client on stdin and stdout until
/// the client disconnects.
///
/// # Errors
/// Returns the failure to start the session or to run it to its end.
pub async fn serve_stdio(docs_dir: &Path) -> Result<(), String> {
    let running = BundleServer::new(docs_dir)
        .serve(stdio())
        .await
        .map_err(|error| error.to_string())?;
    running
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error: tokio::task::JoinError| error.to_string())
}

/// Runs [`serve_stdio`] to completion on a runtime built for the call, for a
/// caller that has no async runtime of its own.
///
/// # Errors
/// Returns the failure to build the runtime, to start the session or to run it
/// to its end.
pub fn serve_stdio_blocking(docs_dir: &Path) -> Result<(), String> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?
        .block_on(serve_stdio(docs_dir))
}
