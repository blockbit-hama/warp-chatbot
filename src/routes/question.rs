use std::collections::HashMap;
use warp::http::StatusCode;

use crate::store::Store;
use crate::model::pagination::extract_pagination;
use crate::model::question::{NewQuestion, Question, QuestionId};
use crate::profanity::check_profanity;
use tracing::{info, instrument};
use handle_errors::Error;
use tokio::join;

#[instrument]
pub async fn get_questions(
    params: HashMap<String, String>,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("querying questions");
    if !params.is_empty() {
        let pagination = extract_pagination(params)?;
        info!(pagination = true);
        let res: Vec<Question> = store.questions.read().await.values().cloned().collect();
        let res = &res[pagination.start..pagination.end];
        Ok(warp::reply::json(&res))
    } else {
        info!(pagination = false);
        let res: Vec<Question> = store.questions.read().await.values().cloned().collect();
        Ok(warp::reply::json(&res))
    }
}

pub async fn add_question(
    store: Store,
    new_question: NewQuestion,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (title_res, content_res) = join!(
        check_profanity(new_question.title.clone()),
        check_profanity(new_question.content.clone())
    );
    
    let title = title_res.map_err(warp::reject::custom)?;
    let content = content_res.map_err(warp::reject::custom)?;
    
    let question = NewQuestion {
        title,
        content,
        tags: new_question.tags,
    };
    
    let saved = store.add_question(question).await
      .map_err(warp::reject::custom)?;
    
    Ok(warp::reply::json(&saved))
}

pub async fn update_question(
    id: String,
    store: Store,
    question: Question,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.questions.write().await.get_mut(&QuestionId(id)) {
        Some(q) => *q = question,
        None => return Err(warp::reject::custom(Error::QuestionNotFound)),
    }
    
    Ok(warp::reply::with_status("Question updated", StatusCode::OK))
}

pub async fn delete_question(
    id: String,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.questions.write().await.remove(&QuestionId(id)) {
        Some(_) => Ok(warp::reply::with_status("Question deleted", StatusCode::OK)),
        None => Err(warp::reject::custom(Error::QuestionNotFound)),
    }
}
