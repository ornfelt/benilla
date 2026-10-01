use crate::{
    app::{App, AppExit},
    plugin::Plugin,
    PluginsState,
};
use bevy_platform::time::Instant;
use core::time::Duration;

/// Determines the method used to run an [`App`]'s [`Schedule`](bevy_ecs::schedule::Schedule).
///
/// It is used in the [`ScheduleRunnerPlugin`].
#[derive(Copy, Clone, Debug)]
pub enum RunMode {
    /// Indicates that the [`App`]'s schedule should run repeatedly.
    Loop {
        /// The minimum [`Duration`] to wait after a [`Schedule`](bevy_ecs::schedule::Schedule)
        /// has completed before repeating. A value of [`None`] will not wait.
        wait: Option<Duration>,
    },
}

impl Default for RunMode {
    fn default() -> Self {
        RunMode::Loop { wait: None }
    }
}

/// Configures an [`App`] to run its [`Schedule`](bevy_ecs::schedule::Schedule) according to a given
/// [`RunMode`].
///
/// [`ScheduleRunnerPlugin`] is included in the
/// [`MinimalPlugins`](https://docs.rs/bevy/latest/bevy/struct.MinimalPlugins.html) plugin group.
///
/// [`ScheduleRunnerPlugin`] is *not* included in the
/// [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html) plugin group
/// which assumes that the [`Schedule`](bevy_ecs::schedule::Schedule) will be executed by other means:
/// typically, the `winit` event loop
/// (see [`WinitPlugin`](https://docs.rs/bevy/latest/bevy/winit/struct.WinitPlugin.html))
/// executes the schedule making [`ScheduleRunnerPlugin`] unnecessary.
#[derive(Default)]
pub struct ScheduleRunnerPlugin {
    /// Determines whether the [`Schedule`](bevy_ecs::schedule::Schedule) is run once or repeatedly.
    pub run_mode: RunMode,
}

impl Plugin for ScheduleRunnerPlugin {
    fn build(&self, app: &mut App) {
        let run_mode = self.run_mode;
        app.set_runner(move |mut app: App| {
            let plugins_state = app.plugins_state();
            if plugins_state != PluginsState::Cleaned {
                while app.plugins_state() == PluginsState::Adding {
                    bevy_tasks::tick_global_task_pools_on_main_thread();
                }
                app.finish();
                app.cleanup();
            }

            match run_mode {
                RunMode::Loop { wait } => {
                    let tick = move |app: &mut App,
                                     _wait: Option<Duration>|
                          -> Result<Option<Duration>, AppExit> {
                        let start_time = Instant::now();

                        app.update();

                        if let Some(exit) = app.should_exit() {
                            return Err(exit);
                        };

                        let end_time = Instant::now();

                        if let Some(wait) = _wait {
                            let exe_time = end_time - start_time;
                            if exe_time < wait {
                                return Ok(Some(wait - exe_time));
                            }
                        }

                        Ok(None)
                    };

                    loop {
                        match tick(&mut app, wait) {
                            Ok(Some(delay)) => {
                                bevy_platform::thread::sleep(delay);
                            }
                            Ok(None) => continue,
                            Err(exit) => return exit,
                        }
                    }
                }
            }
        });
    }
}
