
use egui::*;

pub fn new() -> impl FnMut(&Context) -> () {

  let mut name = "".to_string();
  let mut age = 0;
  let mut show_settings = false;

  move |ctx: &Context| {

    TopBottomPanel::top("top_panel").show(ctx, |ui| {
      // The top panel is often a good place for a menu bar:
      menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {

          if ui.button("Settings").clicked() {
            show_settings = !show_settings;
          }

          if ui.button("Quit").clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
          }
        });
      });
    });

    SidePanel::left("side_panel").show(ctx, |ui| {
      ui.heading("Side Panel");

      ui.horizontal(|ui| {
        ui.label("Write something: ");
        // ui.text_edit_singleline(label);
      });

      // ui.add(Slider::new(value, 0.0..=10.0).text("value"));
      if ui.button("Increment").clicked() {
        // *value += 1.0;
      }

      ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        ui.horizontal(|ui| {
          ui.spacing_mut().item_spacing.x = 0.0;
          ui.label("powered by ");
          ui.hyperlink_to("egui", "https://github.com/emilk/egui");
          ui.label(" and ");
          ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
          );
          ui.label(".");
        });
      });
    });

    CentralPanel::default().show(ctx, |ui| {
      // The central panel the region left after adding TopPanel's and SidePanel's

      ui.hyperlink("https://github.com/emilk/eframe_template");
      warn_if_debug_build(ui);

      ui.heading("My egui Application");
      ui.horizontal(|ui| {
        ui.label("Your name: ");
        ui.text_edit_singleline(&mut name);
      });

      ui.add(Slider::new(&mut age, 0..=120).text("age"));

      if ui.button("Click each year").clicked() {
          age += 1;
      }

      ui.label(format!("Hello '{name}', age {age}"));


      // ui.image(include_image!("../../../wgx/examples/common/img/logo_red.png"));


      /*let painter = ui.painter();

      painter.circle_filled([100.0, 100.0].into(), 50.0, Color32::from_rgb(0xFF, 0x00, 0x00));

      painter.hline(40.0..=600.0, 30.0, Stroke::from((10.0, Color32::from_rgb(0xFF, 0x00, 0x00))));*/


      if show_settings {
        ctx.settings_ui(ui);
      }
    });
  }
}