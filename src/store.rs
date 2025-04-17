use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::model::{
    answer::{Answer, AnswerId},
    question::{Question, QuestionId},
};
use crate::model::question::NewQuestion;
use handle_errors::Error;

#[derive(Debug, Clone)]
pub struct Store {
    pub questions: Arc<RwLock<HashMap<QuestionId, Question>>>,
    pub answers: Arc<RwLock<HashMap<AnswerId, Answer>>>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            questions: Arc::new(RwLock::new(Self::init())),
            answers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn init() -> HashMap<QuestionId, Question> {
        let file = include_str!("../questions.json");
        serde_json::from_str(file).expect("can't read questions.json")
    }
    
    pub async fn add_question(&self, _question: NewQuestion) -> Result<Question, Error> {
       Err(Error::QuestionNotFound)
    }
}
