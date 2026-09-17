use super::*;

impl VcGui {
    pub(crate) fn close_window_ui(&mut self, ctx: &egui::Context) {
        if !self.close_guard.pending() {
            return;
        }
        enum Action {
            None,
            Cancel,
            Minimize,
            Exit,
        }
        let response = egui::Modal::new(egui::Id::new("close-running-audio")).show(ctx, |ui| {
            ui.heading("Keep Sooara running?");
            ui.label("Exiting stops noise cancellation and voice changing.");
            ui.label("Minimize to keep audio running in the taskbar.");
            ui.horizontal(|ui| {
                if ui.button("Keep running / Minimize").clicked() {
                    Action::Minimize
                } else if ui.button("Stop and exit").clicked() {
                    Action::Exit
                } else if ui.button("Cancel").clicked() {
                    Action::Cancel
                } else {
                    Action::None
                }
            })
            .inner
        });
        match response.inner {
            Action::Minimize => {
                self.close_guard.cancel();
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            Action::Exit => {
                self.close_guard.confirm();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Cancel => self.close_guard.cancel(),
            Action::None if response.should_close() => self.close_guard.cancel(),
            Action::None => {}
        }
    }
}
