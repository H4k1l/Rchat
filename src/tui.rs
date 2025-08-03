use ratatui::{self, crossterm::style::Color, layout::{Constraint, Layout}, style::Stylize, widgets::{Block, List, Widget}};
use std::sync::{Arc, Mutex};

pub fn renderthread(messagess: Arc<Mutex<Vec<String>>>, usertype: Arc<Mutex<std::string::String>>, conname: String){ // thread for rendering the TUI
    let mut terminal = ratatui::init();
    loop {
        let _ = terminal.draw(|f| render(f, &messagess.lock().unwrap().to_vec(), &usertype.lock().unwrap(), &conname));
    }
}

pub fn render(frame: &mut ratatui::Frame, messagess: &Vec<String>, usertype: &str, conname: &str) { // render the tui with ratatui

    let border = Layout::default()
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .margin(1)
        .split(frame.area());

    let [inner1] = Layout::vertical([Constraint::Fill(1)])
        .margin(1)
        .areas(border[0]);

    let [inner2] = Layout::vertical([Constraint::Fill(1)])
        .margin(1)
        .areas(border[1]);

    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(format!("Chat with: {}", conname))
        .fg(Color::Green)
        .render(border[0], frame.buffer_mut());

    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title("Type") // chat with {name}...
        .fg(Color::Yellow)
        .render(border[1], frame.buffer_mut());

    let list = List::new(
        messagess.iter().map(|f| f.clone())
    );

    
    frame.render_widget(list, inner1);
    frame.render_widget(usertype, inner2);

}
