use super::item::Item;
use csv;
use indicatif::ProgressBar;

pub trait Lister {
    fn list(&mut self, items: &[Item]);
    fn close(&mut self);
}

pub fn build_lister(item_count: usize) -> impl Lister {
    CSVLister::new(item_count)
}

struct ProgressLister {
    start: std::time::Instant,
    progress: ProgressBar,
    item_count: usize,

    inner: Box<dyn Lister>
}

impl ProgressLister {
    fn new(item_count: usize, inner: impl Lister + 'static) -> ProgressLister {
        let prog = ProgressBar::new(item_count as u64);
        prog.set_message("Fetching records");

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
        self.inner.list(items);
    }
    fn close(&mut self) {
        self.inner.close();

        let msg = format!("Fetched {} items in {}s", self.item_count, self.start.elapsed().as_secs());
        self.progress.finish_with_message(&msg);
    }

}

struct CSVLister {
    start: std::time::Instant,
    progress: ProgressBar,
    item_count: usize,
    items: Vec<Item>,
    writer: csv::Writer<std::io::Stdout>
}

impl CSVLister {

    fn new(item_count: usize) -> Self {
        let prog = ProgressBar::new(item_count as u64);
        prog.set_message("Fetching records");

        CSVLister{
            start: std::time::Instant::now(),
            progress: prog,
            item_count,
            items: Vec::with_capacity(item_count),
            writer: csv::Writer::from_writer(std::io::stdout())
        }
    }

}

impl Lister for CSVLister {


    fn list(&mut self, items: &[Item]) {
        self.items.extend_from_slice(items);
        self.progress.inc(items.len() as u64);
    }

    fn close(&mut self) {
        let msg = format!("Fetched {} items in {}s", self.item_count, self.start.elapsed().as_secs());
        self.progress.finish_with_message(&msg);
        for item in self.items.iter() {
            self.writer.serialize(item).unwrap();
        }
    }

}