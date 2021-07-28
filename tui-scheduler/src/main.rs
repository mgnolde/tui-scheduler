use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, ListState, Paragraph, Row, Table, TableState, Wrap,
    },
    Terminal,
};
use dict::{ Dict, DictIface };
use chrono;







#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    priority: usize,
    name: String,
    category: String,
    description: String,
    begin: DateTime<Local>,
    end: DateTime<Local>,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Entries,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Entries => 1,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");




    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;


    let mut active_menu_item = MenuItem::Entries;
    let mut entry_list_state = ListState::default();
    let mut entry_table_state = TableState::default();
    entry_list_state.select(Some(0));
    entry_table_state.select(Some(0));


    let mut dict = Dict::<Color>::new();
    assert_eq!( dict.add( "bg".to_string(), Color::Rgb(0,38,55) ), true );
    assert_eq!( dict.add( "box_fg".to_string(), Color::Rgb(79,110,121) ), true );
    assert_eq!( dict.add( "box_fg_hl".to_string(), Color::Rgb(0,197,198) ), true );
    assert_eq!( dict.add( "table_header".to_string(), Color::Rgb(193,198,190) ), true );
    assert_eq!( dict.add( "text".to_string(), Color::Rgb(154,184,226) ), true );
    assert_eq!( dict.add( "text_hl".to_string(), Color::Rgb(0,255,255) ), true );
    assert_eq!( dict.add( "progressbar".to_string(), Color::Rgb(0,57,79) ), true );
    assert_eq!( dict.add( "progressbar_hl".to_string(), Color::Rgb(111,146,159) ), true );

    let entry_list = read_db().expect("can fetch entry list");
	let amount_entries = entry_list.len();



    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints(
                    [
                        Constraint::Length(0),
                        Constraint::Min(2),
                        Constraint::Length(0),
                    ]
                    .as_ref(),
                )
                .split(size);

            match active_menu_item {
                MenuItem::Entries => {
                    let entries_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(0), Constraint::Percentage(60), Constraint::Percentage(40)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_entries(&entry_list, &entry_list_state);
                    rect.render_stateful_widget(left, entries_chunks[1], &mut entry_table_state );
                    rect.render_widget(right, entries_chunks[2]);
                }
            }
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('p') => active_menu_item = MenuItem::Entries,
                KeyCode::Down => {
                    if let Some(selected) = entry_table_state.selected() {
                        if selected >= amount_entries - 1 {
                            entry_list_state.select(Some(0));
                            entry_table_state.select(Some(0));
                        } else {
                            entry_list_state.select(Some(selected + 1));
                            entry_table_state.select(Some(selected + 1));
                        }
                    }
                }
                KeyCode::Up => {
                    if let Some(selected) = entry_table_state.selected() {
                        if selected > 0 {
                            entry_list_state.select(Some(selected - 1));
                            entry_table_state.select(Some(selected - 1));
                        } else {
                            entry_list_state.select(Some(amount_entries - 1));
                            entry_table_state.select(Some(amount_entries - 1));
                        }
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}



fn render_entries<'a>(entry_list: &std::vec::Vec<Entry>, entry_list_state: &ListState) -> (Table<'a>, Paragraph<'a>) {

    let mut dict = Dict::<Color>::new();
    assert_eq!( dict.add( "bg".to_string(), Color::Rgb(0,38,55) ), true );
    assert_eq!( dict.add( "box_fg".to_string(), Color::Rgb(79,110,121) ), true );
    assert_eq!( dict.add( "box_fg_hl".to_string(), Color::Rgb(0,197,198) ), true );
    assert_eq!( dict.add( "table_header".to_string(), Color::Rgb(193,198,190) ), true );
    assert_eq!( dict.add( "text".to_string(), Color::Rgb(154,184,226) ), true );
    assert_eq!( dict.add( "text_hl".to_string(), Color::Rgb(0,255,255) ), true );
    assert_eq!( dict.add( "progressbar".to_string(), Color::Rgb(0,57,79) ), true );
    assert_eq!( dict.add( "progressbar_hl".to_string(), Color::Rgb(111,146,159) ), true );


    let tablerows: Vec<_> = entry_list
        .iter()
        .map(|entry| {

			let mut days = Vec::new();

			for i in 0..7 {
				if entry.begin.date() <= chrono::Local::now().date() + chrono::Duration::seconds(i * 24 * 60 * 60) && entry.end.date() >= chrono::Local::now().date() + chrono::Duration::seconds(i * 24 * 60 * 60) {
					days.push("■■■■■■■■■■■■■■■■■■■■■■■■■■■■");
				} else {
					days.push("                            ");
				}
			}

	        Row::new(vec![
	            Cell::from(entry.name.clone()),
	            Cell::from(days[0]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            ),
	            Cell::from(days[1]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            ),
	            Cell::from(days[2]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            ),
	            Cell::from(days[3]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            ),
	            Cell::from(days[4]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            ),
	            Cell::from(days[5]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            ),
	            Cell::from(days[6]).style(Style::default()
					.fg(*dict.get( "text" ).unwrap())
	                .bg(*dict.get( "bg" ).unwrap())
	            )
	        ])
        })
        .collect();


    let selected_entry = entry_list
        .get(
            entry_list_state
                .selected()
                .expect("there is always a selected entry"),
        )
        .expect("exists")
        .clone();





    let home = Table::new(tablerows)
    .style(Style::default()
        .fg(*dict.get( "box_fg_hl" ).unwrap())
        .bg(*dict.get( "bg" ).unwrap())
    )
    .header(
        Row::new(vec![
            Cell::from(""),
            Cell::from(
				chrono::Local::now().format("%a %e").to_string()
				).style(Style::default()
                .fg(*dict.get( "table_header" ).unwrap())
            ),
            Cell::from((chrono::Local::now().date() + chrono::Duration::days(1)).format("%a %e").to_string()).style(Style::default()
                .fg(*dict.get( "table_header" ).unwrap())
            ),
            Cell::from(Spans::from(vec![
                Span::styled((chrono::Local::now().date() + chrono::Duration::days(2)).format("%a %e").to_string(), Style::default()
                    .fg(*dict.get( "table_header" ).unwrap())
                )
            ])),
            Cell::from(Spans::from(vec![
                Span::styled((chrono::Local::now().date() + chrono::Duration::days(3)).format("%a %e").to_string(), Style::default()
                    .fg(*dict.get( "table_header" ).unwrap())
                )
            ])),
            Cell::from(Spans::from(vec![
                Span::styled((chrono::Local::now().date() + chrono::Duration::days(4)).format("%a %e").to_string().to_string(), Style::default()
                    .fg(*dict.get( "table_header" ).unwrap())
                )
            ])),
            Cell::from(Spans::from(vec![
                Span::styled((chrono::Local::now().date() + chrono::Duration::days(5)).format("%a %e").to_string(), Style::default()
                    .fg(*dict.get( "table_header" ).unwrap())
                )
            ])),
            Cell::from(Spans::from(vec![
                Span::styled((chrono::Local::now().date() + chrono::Duration::days(6)).format("%a %e").to_string(), Style::default()
                    .fg(*dict.get( "table_header" ).unwrap())
                )
            ])),
        ])
        .bottom_margin(1)
    )
    .block(Block::default()
        .title("Tasks")
        .borders(Borders::ALL)
    )
    .widths(&[
		Constraint::Percentage(23), 
		Constraint::Percentage(11), 
		Constraint::Percentage(11), 
		Constraint::Percentage(11), 
		Constraint::Percentage(11), 
		Constraint::Percentage(11), 
		Constraint::Percentage(11), 
		Constraint::Percentage(11)
	])
    .column_spacing(0)
    .highlight_style(
        Style::default()
        .bg(*dict.get( "progressbar" ).unwrap())
        .fg(*dict.get( "text_hl" ).unwrap())
        .add_modifier(Modifier::BOLD)
    )
    ;



	let text = vec![

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("Name",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled(selected_entry.name.to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("Category",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled(selected_entry.category.to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("Priority",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled(selected_entry.priority.to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("Description",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled(selected_entry.description.to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("Begin",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled(selected_entry.begin.to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("End",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled(selected_entry.end.to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),

	    Spans::from(Span::styled("", Style::default().fg(Color::Red))),
	    Spans::from(Span::styled("Elapsed days",   Style::default().fg(*dict.get( "table_header" ).unwrap()))),
	    Spans::from(Span::styled((chrono::Local::now().date() - selected_entry.begin.date()).num_days().to_string(), Style::default().fg(*dict.get( "text_hl" ).unwrap()))),
	];

    let entry_detail = Paragraph::new(text)
        .style(Style::default()
			.fg(*dict.get( "box_fg_hl" ).unwrap())
			.bg(*dict.get( "bg" ).unwrap())
		)
		.wrap(Wrap { trim: true })
        .alignment(Alignment::Left)
        .block(
			Block::default()
				.borders(Borders::ALL)
				.title("Details")
				.style(Style::default()
					.fg(*dict.get( "box_fg_hl" ).unwrap())
					.bg(*dict.get( "bg" ).unwrap())
				)
				.border_type(BorderType::Plain),
        );


    (home, entry_detail)
}

fn read_db() -> Result<Vec<Entry>, Error> {
	
	
	match home::home_dir() {

		Some(path) => {
	
			let path_str = path.display().to_string();
	
			let mut path_prefix: String = path_str.to_owned();
			let path_suffix: &str = "/tasks.json";
	
			path_prefix.push_str(path_suffix);
		
			
		    let db_content = fs::read_to_string(path_prefix)?;
		    let mut parsed: Vec<Entry> = serde_json::from_str(&db_content)?;
		
			let mut i = 0;	
			let mut remove_ids = Vec::new();
		
		
			for name in parsed.iter() {
				if name.begin.date() > chrono::Local::now().date() + chrono::Duration::seconds(7 * 24 * 60 * 60) {
					remove_ids.push(i);
				}
		
				if name.end.date() < chrono::Local::now().date() {
					remove_ids.push(i);
				}
		
				i = i + 1;
		
			}
		
			
			for id in remove_ids.iter().rev() {	
				parsed.remove(*id);
			}
	
		    Ok(parsed)
		},
		None =>  {
			let path_str = "".to_string();
			let parsed: Vec<Entry> = serde_json::from_str(&path_str)?;
			Ok(parsed)
			},
	}
		
	


}

