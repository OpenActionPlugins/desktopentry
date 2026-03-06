use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use base64::{Engine as _, engine::general_purpose};
use freedesktop_desktop_entry::{DesktopEntry, Iter, default_paths};
use freedesktop_icons::lookup;
use openaction::*;
use serde::{Deserialize, Serialize};

fn is_flatpak() -> bool {
	use std::env::var;
	var("FLATPAK_ID").is_ok()
		|| var("container")
			.map(|x| x.to_lowercase().trim() == "flatpak")
			.unwrap_or(false)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppInfo {
	pub path: String,
	pub name: String,
	pub exec: String,
	pub icon: Option<String>,
	pub terminal: bool,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct LaunchAppSettings {
	pub app: Option<String>,
	pub args: Option<String>,
}

pub struct LaunchAppAction;

#[async_trait]
impl Action for LaunchAppAction {
	const UUID: ActionUuid = "me.amankhanna.oadesktopentry.launchapp";
	type Settings = LaunchAppSettings;

	async fn will_appear(
		&self,
		instance: &Instance,
		_settings: &Self::Settings,
	) -> OpenActionResult<()> {
		send_apps_to_pi(instance).await
	}

	async fn did_receive_settings(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		update_icon(instance, settings).await
	}

	async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
		send_apps_to_pi(instance).await?;
		launch_app(settings.app.as_deref(), settings.args.as_deref());
		Ok(())
	}

	async fn dial_up(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_up(instance, settings).await
	}
}

fn get_icon_path_from_desktop_entry(desktop_entry_path: impl Into<PathBuf>) -> Option<PathBuf> {
	let locales: &[&str] = &[];
	let entry = DesktopEntry::from_path(desktop_entry_path, locales.into()).ok()?;
	let icon = entry.icon()?.to_owned();
	lookup(&icon).with_size(256).find()
}

async fn update_icon(instance: &Instance, settings: &LaunchAppSettings) -> OpenActionResult<()> {
	let icon = settings
		.app
		.as_deref()
		.filter(|s| !s.is_empty())
		.and_then(get_icon_path_from_desktop_entry)
		.and_then(|path| icon_to_base64(&path))
		.unwrap_or_else(|| "icon".to_owned());

	instance.set_image(Some(icon), None).await
}

async fn send_apps_to_pi(instance: &Instance) -> OpenActionResult<()> {
	let apps = get_installed_apps();
	log::debug!("Sending {} apps to property inspector", apps.len());
	instance
		.send_to_property_inspector(serde_json::json!({ "apps": apps }))
		.await
}

fn launch_app(app_path: Option<&str>, custom_args: Option<&str>) {
	let Some(app) = app_path.filter(|s| !s.is_empty()).and_then(find_app) else {
		return;
	};

	let exec = app
		.exec
		.replace("%u", "")
		.replace("%U", "")
		.replace("%f", "")
		.replace("%F", "")
		.replace("%i", "")
		.replace("%c", "")
		.replace("%k", "")
		.replace("%%", "%");
	let exec = exec.trim();

	let full_command = match custom_args.filter(|s| !s.trim().is_empty()) {
		Some(args) => format!("{} {}", exec, args),
		None => exec.to_owned(),
	};

	let full_command = if app.terminal {
		let terminals = [
			("x-terminal-emulator", "-e"),
			("gnome-terminal", "--"),
			("konsole", "-e"),
			("xfce4-terminal", "-e"),
			("mate-terminal", "-e"),
			("tilix", "-e"),
			("alacritty", "-e"),
			("kitty", "--"),
			("foot", ""),
			("xterm", "-e"),
		];
		let (terminal, flag) = terminals
			.iter()
			.find(|(t, _)| {
				Command::new("which")
					.arg(t)
					.output()
					.is_ok_and(|o| o.status.success())
			})
			.unwrap_or_else(|| {
				log::warn!("No terminal emulator found, defaulting to xterm");
				&("xterm", "-e")
			});
		if flag.is_empty() {
			format!("{} {}", terminal, full_command)
		} else {
			format!("{} {} {}", terminal, flag, full_command)
		}
	} else {
		full_command
	};

	let (command, args): (&str, Vec<&str>) = if is_flatpak() {
		("flatpak-spawn", vec!["--host", "sh", "-c", &full_command])
	} else {
		("sh", vec!["-c", &full_command])
	};

	log::info!("Launching: {}", full_command);
	if let Err(e) = Command::new(command)
		.args(args)
		.current_dir(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_owned()))
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
	{
		log::error!("Failed to launch {}: {e}", full_command);
	}
}

fn find_app(path: &str) -> Option<AppInfo> {
	let locales: &[&str] = &[];

	let path = Path::new(path);
	let entry = DesktopEntry::from_path(path, locales.into()).ok()?;

	Some(AppInfo {
		path: path.to_string_lossy().into_owned(),
		name: entry.name(locales)?.into_owned(),
		exec: entry.exec()?.to_owned(),
		icon: entry.icon().map(|s| s.to_owned()),
		terminal: entry.terminal(),
	})
}

fn get_installed_apps() -> Vec<AppInfo> {
	let locales: &[&str] = &[];

	let mut apps: Vec<AppInfo> = Iter::new(default_paths())
		.filter_map(|path| {
			let entry = DesktopEntry::from_path(&path, locales.into()).ok()?;

			if entry.no_display() || entry.hidden() {
				return None;
			}

			Some(AppInfo {
				path: path.to_string_lossy().into_owned(),
				name: entry.name(locales)?.into_owned(),
				exec: entry.exec()?.to_owned(),
				icon: entry.icon().map(|s| s.to_owned()),
				terminal: entry.terminal(),
			})
		})
		.collect();

	apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
	apps
}

fn icon_to_base64(path: &Path) -> Option<String> {
	let data = fs::read(path).ok()?;
	let mime = match path.extension()?.to_str()? {
		"svg" => "svg+xml",
		"png" => "png",
		"xpm" => "x-xpixmap",
		"ico" => "x-icon",
		"jpg" | "jpeg" => "jpeg",
		_ => path.extension()?.to_str()?,
	};
	Some(format!(
		"data:image/{mime};base64,{}",
		general_purpose::STANDARD.encode(&data)
	))
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
	{
		use simplelog::*;
		if let Err(error) = TermLogger::init(
			LevelFilter::Debug,
			Config::default(),
			TerminalMode::Stdout,
			ColorChoice::Never,
		) {
			eprintln!("Logger initialization failed: {}", error);
		}
	}

	register_action(LaunchAppAction).await;

	run(std::env::args().collect()).await
}
