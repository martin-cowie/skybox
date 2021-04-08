use super::item::Item;
use csv;
use indicatif::ProgressBar;

pub trait Lister {
    fn list(&mut self, items: &[Item]);
    fn close(&mut self);
}

/**
 * Factory: build a lister
 */
pub fn build_lister(item_count: usize, matches: &clap::ArgMatches) -> impl Lister {
    //TODO: refactor -  ProgressLister::new is repeated
    match matches.value_of("FORMAT") {
        Some("JSON") => ProgressLister::new(item_count, JSONLister::new(item_count)),
        Some("CSV") => ProgressLister::new(item_count, CSVLister::new(item_count)),
        _ => ProgressLister::new(item_count, SimpleLister::new(item_count, matches.clone()))
    }
}


//===========

/**
 * Wrap another Lister and drive a progress bar
 */
struct ProgressLister {
    start: std::time::Instant,
    progress: ProgressBar,
    item_count: usize,

    inner: Box<dyn Lister>
}

impl ProgressLister {
    fn new(item_count: usize, inner: impl Lister + 'static) -> Self {
        let prog = ProgressBar::new(item_count as u64);
        prog.println("Fetching recordings from skybox");

        ProgressLister {
            start: std::time::Instant::now(),
            progress: prog,
            item_count,
            inner: Box::new(inner)
        }
    }

}

impl Lister for ProgressLister {

    fn list(&mut self, items: &[Item]) {
        self.progress.inc(items.len() as u64);
        self.inner.list(items);
    }
    fn close(&mut self) {
        self.inner.close();

        let msg = format!("Fetched {} items in {}s", self.item_count, self.start.elapsed().as_secs());
        self.progress.finish_with_message(&msg);
    }

}

//===========

/**
 * Output Items as CSV
 */
struct CSVLister {
    items: Vec<Item>,
    writer: csv::Writer<std::io::Stdout>
}

impl CSVLister {

    fn new(item_count: usize) -> Self {
        CSVLister{
            items: Vec::with_capacity(item_count),
            writer: csv::Writer::from_writer(std::io::stdout())
        }
    }

}

impl Lister for CSVLister {

    fn list(&mut self, items: &[Item]) {
        self.items.extend_from_slice(items);
    }

    fn close(&mut self) {
        for item in self.items.iter() {
            self.writer.serialize(item).unwrap();
        }
    }

}

/**
 * Output Items as JSON
 */

struct JSONLister {
    items: Vec<Item>
}

impl JSONLister {
    fn new(item_count: usize) -> Self {
        JSONLister{
            items: Vec::with_capacity(item_count),
        }
    }
}

impl Lister for JSONLister {
    fn list(&mut self, items: &[Item]) {
        self.items.extend_from_slice(items);
    }

    fn close(&mut self) {
        println!("{}", serde_json::to_string(&self.items).expect("Cannot serialise result"));
    }
}

/**
 * Output Items a text
 */
struct SimpleLister {
    items: Vec<Item>,
    matches: clap::ArgMatches
}

impl SimpleLister {
    fn new(item_count: usize, matches: clap::ArgMatches) -> Self {
        SimpleLister{
            items: Vec::with_capacity(item_count),
            matches: matches
        }
    }
}

impl Lister for SimpleLister {
    fn list(&mut self, items: &[Item]) {
        self.items.extend_from_slice(items);
    }

    fn close(&mut self) {

        // if self.matches.is_present("TIME_ORDER") {
        //     if self.matches.is_present("REVERSE_TIME") {
        //         self.items.sort_by(|a,b| a.recorded_starttime < b.recorded_starttime)
        //     } else {

        //     }
        // }

        for item in self.items.iter() {
            println!("{} {} {}: {}",
                item.recorded_starttime,
                item.recorded_duration,
                item.title,
                item.description
            );
        }
    }
}