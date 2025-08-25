// importing modules
use crate::connections::ConnEvent;

// importing libraries
use std::{
    sync::{Arc},
};

use ratatui::{
    self, 
    crossterm::style::Color, 
    layout::{
        Constraint, 
        Layout, 
        Margin
    }, 
    style::{
        Style, 
        Stylize
    }, 
    text::Line, 
    widgets::{
        Block, 
        List, 
        ListState, 
        Paragraph, 
        Scrollbar, 
        ScrollbarState, 
        Widget
    }
};

use tokio::{
    sync::{Mutex}    
};

pub async fn renderthread(messages: Arc<Mutex<Vec<String>>>, usertype: Arc<Mutex<String>>, scrollpos: Arc<Mutex<u16>>, view_height: Arc<Mutex<u16>>, conname: String, event: Arc<Mutex<ConnEvent>>){ // thread for rendering the TUI
   
    let mut terminal = ratatui::init();
    
    loop {
        
        let messages = messages.lock().await.to_vec();
        let usertype = usertype.lock().await.clone();
        let scrollpos = scrollpos.lock().await.clone();
        let event = event.lock().await.clone();

        let mut view_height_draw: u16 = 0;

        let _ = terminal.draw(|f| { // start drawing the terminal
    
            let border = Layout::default()
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .margin(1)
                .split(f.area());

            let [inner] = Layout::vertical([Constraint::Fill(1)])
                .margin(1)
                .areas(border[0]);

            view_height_draw = inner.height;
        
            let _ = border;
            let _ = inner;

            render(f, messages.clone(), &usertype, scrollpos, &conname, event);
        });

        *view_height.lock().await = view_height_draw; // get the height of the frame for calculating the scroll bar

    }

}

fn render(frame: &mut ratatui::Frame, messagess: Vec<String>, usertype: &str, scrollpos: u16, conname: &str, event: ConnEvent) { // render the tui with ratatui

    let border = Layout::default() // dividing the principal terminal with 80% messages log and 20% user type area
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .margin(1)
        .split(frame.area());

    let innerborder = Layout::horizontal([Constraint::Fill(1)]) // dividing the messages log area 90% with messages and 10% with commands
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)])
        .margin(1)
        .split(border[0]);

    let [inner1] = Layout::vertical([Constraint::Fill(1)])
        .margin(1)
        .areas(innerborder[0]);

    let [inner2] = Layout::vertical([Constraint::Fill(1)])
        .margin(1)
        .areas(border[1]);

    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(format!(" Chat with: {} ", conname)) // Chat with {name}...
        .fg(Color::Green)
        .render(border[0], frame.buffer_mut());

    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Type ")
        .fg(Color::Yellow)
        .render(border[1], frame.buffer_mut());

    // defining and creating the messages paragraph 
    let messagess: Vec<Line> = messagess.iter().map(|f| { 
        let style = if !f.starts_with(conname) && !f.starts_with("!"){
            Style::default().fg(ratatui::prelude::Color::LightYellow) // user msg = yellow
        }
        else if f.starts_with("!") {
            Style::default().fg(ratatui::prelude::Color::Red) // error = red
        }
        else {
            Style::default() // default = green
        };
        Line::from(f.clone()).style(style)
    }).collect();

    let message_log = Paragraph::new(messagess.clone()).wrap(ratatui::widgets::Wrap { trim: true });
    let usertype = Paragraph::new(usertype).wrap(ratatui::widgets::Wrap { trim: true });

    // defining and creating the scrollbar
    let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let mut scrollbar_state = ScrollbarState::new(messagess.len()).position(scrollpos as usize);

    let popup_height = inner1.height / 3;

    // event handler
    match event {
        ConnEvent::None => {
            // the classic behaviour
            let message_log = Paragraph::new(messagess.clone())
                .scroll((scrollpos, 0))
                .wrap(ratatui::widgets::Wrap { trim: true });
            frame.render_widget(usertype, inner2);
            frame.render_widget(message_log, inner1);
            frame.render_stateful_widget(scrollbar, inner1.inner(Margin { horizontal: 0, vertical: 1 }), &mut scrollbar_state);
        },
        ConnEvent::SendFile => { 
            // render a small box requiring the path of the file
            let width = inner1.inner(Margin { horizontal: 60, vertical: popup_height });

            Block::bordered()
                .border_type(ratatui::widgets::BorderType::Plain)
                .title(" Enter the path of the file ")
                .fg(Color::Green)
                .title_alignment(ratatui::layout::Alignment::Center)
                .render(width, frame.buffer_mut());

            let typer = Paragraph::from(usertype)
                .centered();

            frame.render_widget(message_log, inner1);
            frame.render_widget(typer, width.inner(Margin { horizontal: 2, vertical: 2 }));
        },
        ConnEvent::ReceiveFile => {
            // render a small box with an y/n choice
            let width = inner1.inner(Margin { horizontal: 60, vertical: popup_height });

            Block::bordered()
                .border_type(ratatui::widgets::BorderType::Plain)
                .title(format!(" {conname} want to send you a file, accept? "))
                .fg(Color::Green)
                .title_alignment(ratatui::layout::Alignment::Center)
                .render(width, frame.buffer_mut());
                
            let choicevec: Vec<String> = vec!["yes", "no"].iter().map(|c| {
                let padding = (width.width.saturating_sub(2)) / 2 - width.width / 18;
                let spaces = " ".repeat(padding as usize);
                format!("{spaces}{c}")
            }).collect();

            let selection = List::new(choicevec)
                .highlight_symbol(">")
                .highlight_style(Style::default().reversed());

            let mut choiche_state = ListState::default();
            if scrollpos == 0 {
                choiche_state.select(Some(0));
            }
            else if scrollpos == 1 {
                choiche_state.select(Some(1));  
            }
            frame.render_widget(message_log, inner1);
            frame.render_stateful_widget(selection, width.inner(Margin { horizontal: 2, vertical: 2 }), &mut choiche_state);

        }
    }

    // rendering the command helper
    let [inner1cmds] = Layout::vertical([Constraint::Fill(1)])
        .margin(1)
        .areas(innerborder[1]);

    let commands = vec!["!clear".to_string(), "- clear the msg".to_string(), "!sendfile".to_string(), "- send file".to_string(), "!cancel/!cl".to_string(), "- cancel action".to_string()];

    let commands: Vec<Line> = commands.iter().map(|f| {
        Line::from(f.clone())
    }).collect();

    let commands_log = Paragraph::new(commands.clone()).wrap(ratatui::widgets::Wrap { trim: true });

    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Commands ")
        .fg(Color::Yellow)
        .render(innerborder[1], frame.buffer_mut());

    frame.render_widget(commands_log, inner1cmds);

}
