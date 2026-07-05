use log::{debug, trace};
use serde::{Deserialize, Serialize};

use crate::llm::ToolClient;
use crate::types::Result;

#[derive(Serialize)]
pub(crate) struct Application<'a> {
    name: &'a str,
    version: &'a str,
    description: &'a str,
}

#[derive(Serialize)]
pub(crate) struct Update<'a> {
    title: &'a str,
    description: &'a str,
}

impl<'a> Update<'a> {
    fn new(title: &'a str, description: &'a str) -> Self {
        Update { title, description }
    }
}

#[derive(Debug, PartialEq, Serialize)]
enum UpdateType {
    BugFix,
    _NewFeature,
    _PerformanceImprovement,
    _SecurityPatch,
    _UiUxEnhancement,
    _CompatibilityUpdate,
    _Hotfix,
}

#[derive(Serialize)]
pub(crate) struct ReleaseNote<'a> {
    app_name: &'a str,
    app_version: &'a str,
    update_type: &'a UpdateType,
    upates: Vec<Update<'a>>,
}

pub(crate) struct AppManager<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> AppManager<'a> {
    pub(crate) fn new(tool_client: &'a ToolClient) -> Self {
        Self { tool_client }
    }

    pub(crate) async fn exec(&self, prompt: &str) -> Result<String> {
        trace!("AppManager::exec(&self, prompt: &str) -> Result<String>");
        let mut function: Function = self.tool_client.get_function(prompt, "app-manager").await?;
        function.exec().await
    }
}

#[derive(Deserialize)]
#[serde(tag = "function")]
enum Function {
    #[serde(rename = "find_apps")]
    FindApps(FindApps),
    #[serde(rename = "install_app")]
    InstallApp(InstallApp),
    #[serde(rename = "update_app")]
    UpdateApp(UpdateApp),
    #[serde(rename = "uninstall_app")]
    UninstallApp(UninstallApp),
    #[serde(rename = "list_installed_apps")]
    ListInstalledApps(ListInstalledApps),
}

impl Function {
    async fn exec(&mut self) -> Result<String> {
        match self {
            Function::FindApps(call) => call.exec().await,
            Function::InstallApp(call) => call.exec().await,
            Function::UpdateApp(call) => call.exec().await,
            Function::UninstallApp(call) => call.exec().await,
            Function::ListInstalledApps(call) => call.exec().await,
        }
    }
}

// --------------------------------------------------------
// Applications Manager Service

#[derive(Deserialize)]
struct FindApps {
    store_name: String,
    app_description: String,
    limit: u8,
}

impl FindApps {
    pub async fn exec(&self) -> Result<String> {
        trace!("FindApps::exec(&self) -> Result<String>");
        debug!("store_name: {}", self.store_name);
        debug!("app_description: {}", self.app_description);
        debug!("limit: {}", self.limit);

        let app1 = Application {
            name: "My Weather",
            version: "1.0",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let app2 = Application {
            name: "Weather Info",
            version: "1.1",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let apps = vec![app1, app2];
        let result = serde_json::to_string(&apps)?;
        Ok(result)
    }
}

#[derive(Deserialize)]
struct InstallApp {
    store_name: String,
    app_name: String,
}

impl InstallApp {
    pub async fn exec(&self) -> Result<String> {
        trace!("InstallApp::exec(&self) -> Result<String>");
        debug!("store_name: {}", self.store_name);
        debug!("app_name: {}", self.app_name);

        let app = Application {
            name: &self.app_name,
            version: "1.0",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let result = serde_json::to_string(&app)?;
        Ok(result)
    }
}

#[derive(Deserialize)]
struct UpdateApp {
    app_name: String,
}

impl UpdateApp {
    pub async fn exec(&self) -> Result<String> {
        trace!("UpdateApp::exec(&self) -> Result<String>");
        debug!("app_name: {}", self.app_name);

        let u1 = Update::new(
            "Improved Stability",
            "Fixed an issue where the app would occasionally close unexpectedly during complex calculations.",
        );
        let u2 = Update::new(
            "Performance Optimization",
            "Enhanced calculation speed for large-scale scientific functions.",
        );
        let u3 = Update::new(
            "UI Refinement",
            "Minor adjustments to the button layout for better usability on smaller screens.",
        );
        let u4 = Update::new(
            "Bug Fixes",
            "Resolved a display error in the calculation history log.",
        );
        let updates = vec![u1, u2, u3, u4];

        let release_note = ReleaseNote {
            app_name: &self.app_name,
            app_version: "1.2.3",
            update_type: &UpdateType::BugFix,
            upates: updates,
        };
        let result = serde_json::to_string(&release_note)?;
        Ok(result)
    }
}

#[derive(Deserialize)]
struct UninstallApp {
    app_name: String,
}

impl UninstallApp {
    pub async fn exec(&self) -> Result<String> {
        trace!("UninstallApp::exec(&self) -> Result<String>");
        debug!("app_name: {}", self.app_name);

        let app = Application {
            name: &self.app_name,
            version: "1.0",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let result = serde_json::to_string(&app)?;
        Ok(result)
    }
}

#[derive(Deserialize)]
struct ListInstalledApps {
    filter: String,
    limit: u8,
}

impl ListInstalledApps {
    pub async fn exec(&self) -> Result<String> {
        trace!("ListInstalledApps::exec(&self) -> Result<String>");
        debug!("filter: {}", self.filter);
        debug!("limit: {}", self.limit);

        let app1 = Application {
            name: "Angry Birds",
            version: "1.0",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let app2 = Application {
            name: "My Weather",
            version: "1.0",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let app3 = Application {
            name: "Weather Info",
            version: "1.1",
            description: "The 'Angry Birds' app is a popular, high-energy game where players use a slingshot to launch colorful birds at mischievous pigs.",
        };
        let apps = vec![app1, app2, app3];
        let result = serde_json::to_string(&apps)?;
        Ok(result)
    }
}
