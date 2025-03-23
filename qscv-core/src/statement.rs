use crate::database::Database;


pub trait Statement<'q>: Send + Sync {
    type Database: Database;

}

