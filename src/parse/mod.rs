mod grammar;

use std::{borrow::Cow, collections::HashMap};

pub struct TranslationUnit<'i> {
    name: Cow<'i, str>,

    rooms: HashMap<Cow<'i, str>, Room<'i>>,
}

pub struct Room<'i> {
    name: Cow<'i, str>,
}
