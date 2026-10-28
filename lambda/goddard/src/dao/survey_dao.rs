use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::{ApiResult, AppError};
use crate::models::schema::UserRole;
use crate::models::survey::*;

#[derive(Clone)]
pub struct SurveyDao {
    pool: Pool,
}

impl SurveyDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_list_item(row: &Row) -> SurveyListItem {
        SurveyListItem {
            id:             row.get("id"),
            school_id:      row.get("school_id"),
            title:          row.get("title"),
            description:    row.get("description"),
            audience:       row.get("audience"),
            classroom_id:   row.get("classroom_id"),
            classroom_name: row.get("classroom_name"),
            status:         row.get("status"),
            is_anonymous:   row.get("is_anonymous"),
            is_template:    row.get("is_template"),
            expires_at:     row.get("expires_at"),
            published_at:   row.get("published_at"),
            created_at:     row.get("created_at"),
            created_by_name: row.get("created_by_name"),
            total_targeted: row.get("total_targeted"),
            total_responded: row.get("total_responded"),
            question_count:  row.get("question_count"),
        }
    }

    fn row_to_question_dto(row: &Row) -> SurveyQuestionDto {
        let options: Option<Vec<String>> = row
            .get::<_, Option<serde_json::Value>>("options")
            .and_then(|v| serde_json::from_value(v).ok());
        SurveyQuestionDto {
            id:            row.get("id"),
            question_type: row.get("question_type"),
            question_text: row.get("question_text"),
            is_required:   row.get("is_required"),
            options,
            rating_max:    row.get("rating_max"),
            sort_order:    row.get("sort_order"),
        }
    }

    // ── Shared SELECT fragment ─────────────────────────────────────────────────

    const SURVEY_SELECT: &'static str = r#"
        SELECT
            s.id, s.school_id, s.title, s.description, s.audience, s.classroom_id,
            cl.name AS classroom_name,
            CASE
                WHEN s.status = 'published'
                     AND s.expires_at IS NOT NULL
                     AND s.expires_at < NOW()
                THEN 'expired'
                ELSE s.status
            END AS status,
            s.is_anonymous, s.is_template,
            s.expires_at, s.published_at, s.created_at, s.total_targeted,
            u.first_name || ' ' || u.last_name AS created_by_name,
            (SELECT COUNT(*)::bigint FROM survey_responses WHERE survey_id = s.id)  AS total_responded,
            (SELECT COUNT(*)::bigint FROM survey_questions WHERE survey_id = s.id)  AS question_count
        FROM surveys s
        LEFT JOIN classrooms cl ON cl.id = s.classroom_id
        LEFT JOIN users u ON u.id = s.created_by
    "#;

    // ── Create ────────────────────────────────────────────────────────────────

    pub async fn create_survey(
        &self,
        req: &CreateSurveyRequest,
        created_by: Uuid,
    ) -> ApiResult<SurveyDetail> {
        let mut client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let tx = client.transaction().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let status = req.status.as_deref().unwrap_or("draft");
        let published_at: Option<DateTime<Utc>> =
            if status == "published" { Some(Utc::now()) } else { None };

        let row = tx.query_one(
            r#"INSERT INTO surveys
               (school_id, title, description, audience, classroom_id, status,
                is_anonymous, is_template, expires_at, published_at, created_by, created_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW())
               RETURNING id"#,
            &[
                &req.school_id, &req.title, &req.description, &req.audience,
                &req.classroom_id, &status, &req.is_anonymous, &req.is_template,
                &req.expires_at, &published_at, &created_by,
            ],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let survey_id: Uuid = row.get(0);

        for (i, q) in req.questions.iter().enumerate() {
            let options_json: Option<serde_json::Value> = q.options.as_ref()
                .map(|o| serde_json::to_value(o).unwrap_or(serde_json::Value::Null));
            tx.execute(
                r#"INSERT INTO survey_questions
                   (survey_id, question_type, question_text, is_required, options, rating_max, sort_order)
                   VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
                &[
                    &survey_id, &q.question_type, &q.question_text, &q.is_required,
                    &options_json, &q.rating_max, &(i as i32),
                ],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_survey_detail(survey_id, None).await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve created survey".to_string()))
    }

    // ── List surveys (admin management view) ─────────────────────────────────

    pub async fn list_surveys(&self, query: &SurveyListQuery) -> ApiResult<Vec<SurveyListItem>> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;

        let sql = format!(
            "{} WHERE s.school_id = $1 AND s.is_template = false \
               AND ($2::text IS NULL OR s.status = $2) \
               AND ($3::text IS NULL OR s.audience = $3) \
             ORDER BY s.created_at DESC",
            Self::SURVEY_SELECT
        );

        let rows = client.query(
            &sql,
            &[&query.school_id, &query.status, &query.audience],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(Self::row_to_list_item).collect())
    }

    // ── List templates ────────────────────────────────────────────────────────

    pub async fn list_templates(&self, school_id: Uuid) -> ApiResult<Vec<SurveyDetail>> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;

        let sql = format!(
            "{} WHERE s.school_id = $1 AND s.is_template = true ORDER BY s.title ASC",
            Self::SURVEY_SELECT
        );

        let rows = client.query(&sql, &[&school_id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        let mut details = Vec::new();
        for row in &rows {
            let item = Self::row_to_list_item(row);
            let survey_id: Uuid = item.id;
            let q_rows = client.query(
                "SELECT id, question_type, question_text, is_required, options, rating_max, sort_order \
                 FROM survey_questions WHERE survey_id = $1 ORDER BY sort_order ASC",
                &[&survey_id],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
            let questions = q_rows.iter().map(Self::row_to_question_dto).collect();
            details.push(SurveyDetail { survey: item, questions, has_responded: None });
        }
        Ok(details)
    }

    // ── Get survey detail (survey + questions + responded check) ──────────────

    pub async fn get_survey_detail(
        &self,
        id: Uuid,
        user_id: Option<Uuid>,
    ) -> ApiResult<Option<SurveyDetail>> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;

        let sql = format!("{} WHERE s.id = $1", Self::SURVEY_SELECT);
        let survey_row = client.query_opt(&sql, &[&id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        let survey_row = match survey_row {
            None => return Ok(None),
            Some(r) => r,
        };
        let item = Self::row_to_list_item(&survey_row);

        let q_rows = client.query(
            "SELECT id, question_type, question_text, is_required, options, rating_max, sort_order \
             FROM survey_questions WHERE survey_id = $1 ORDER BY sort_order ASC",
            &[&id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        let questions = q_rows.iter().map(Self::row_to_question_dto).collect();

        let has_responded = if let Some(uid) = user_id {
            let resp = client.query_opt(
                "SELECT id FROM survey_responses WHERE survey_id = $1 AND respondent_id = $2",
                &[&id, &uid],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
            Some(resp.is_some())
        } else {
            None
        };

        Ok(Some(SurveyDetail { survey: item, questions, has_responded }))
    }

    // ── Update survey ─────────────────────────────────────────────────────────

    pub async fn update_survey(&self, req: &UpdateSurveyRequest) -> ApiResult<SurveyDetail> {
        let mut client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let tx = client.transaction().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute(
            r#"UPDATE surveys SET
               title       = COALESCE($2, title),
               description = COALESCE($3, description),
               audience    = COALESCE($4, audience),
               classroom_id = COALESCE($5, classroom_id),
               is_anonymous = COALESCE($6, is_anonymous),
               is_template  = COALESCE($7, is_template),
               expires_at   = COALESCE($8, expires_at),
               updated_at   = NOW()
               WHERE id = $1"#,
            &[
                &req.id, &req.title, &req.description, &req.audience,
                &req.classroom_id, &req.is_anonymous, &req.is_template, &req.expires_at,
            ],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(questions) = &req.questions {
            tx.execute(
                "DELETE FROM survey_questions WHERE survey_id = $1",
                &[&req.id],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;

            for (i, q) in questions.iter().enumerate() {
                let options_json: Option<serde_json::Value> = q.options.as_ref()
                    .map(|o| serde_json::to_value(o).unwrap_or(serde_json::Value::Null));
                tx.execute(
                    r#"INSERT INTO survey_questions
                       (survey_id, question_type, question_text, is_required, options, rating_max, sort_order)
                       VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
                    &[
                        &req.id, &q.question_type, &q.question_text, &q.is_required,
                        &options_json, &q.rating_max, &(i as i32),
                    ],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_survey_detail(req.id, None).await?
            .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))
    }

    // ── Delete survey ─────────────────────────────────────────────────────────

    pub async fn delete_survey(&self, id: Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        client.execute("DELETE FROM surveys WHERE id = $1", &[&id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ── Publish survey ────────────────────────────────────────────────────────

    pub async fn publish_survey(&self, id: Uuid, total_targeted: i32) -> ApiResult<SurveyDetail> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let now = Utc::now();
        client.execute(
            "UPDATE surveys SET status = 'published', published_at = $2, total_targeted = $3, updated_at = $2 WHERE id = $1",
            &[&id, &now, &total_targeted],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_survey_detail(id, None).await?
            .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))
    }

    // ── Close survey ──────────────────────────────────────────────────────────

    pub async fn close_survey(&self, id: Uuid) -> ApiResult<SurveyDetail> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let now = Utc::now();
        client.execute(
            "UPDATE surveys SET status = 'closed', updated_at = $2 WHERE id = $1",
            &[&id, &now],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_survey_detail(id, None).await?
            .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))
    }

    // ── Reopen survey ─────────────────────────────────────────────────────────

    pub async fn reopen_survey(&self, id: Uuid) -> ApiResult<SurveyDetail> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let now = Utc::now();
        client.execute(
            "UPDATE surveys SET status = 'published', updated_at = $2 WHERE id = $1",
            &[&id, &now],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_survey_detail(id, None).await?
            .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))
    }

    // ── List surveys for current user (pending/completed) ─────────────────────

    pub async fn list_my_surveys(
        &self,
        user_id: Uuid,
        school_id: Uuid,
        role: &UserRole,
    ) -> ApiResult<Vec<MySurveyItem>> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;

        let audience_clause = match role {
            UserRole::Parent => {
                r#"(s.audience IN ('parents', 'everyone')
                    OR (s.audience = 'classroom' AND EXISTS (
                        SELECT 1 FROM children c
                        JOIN enrollments e ON e.child_id = c.id
                        WHERE (c.parent_id = $1 OR c.secondary_parent_id = $1)
                          AND e.classroom_id = s.classroom_id
                          AND c.school_id = $2
                    )))"#
            }
            UserRole::Teacher => "s.audience IN ('employees', 'all_staff', 'everyone')",
            UserRole::Admin | UserRole::SuperAdmin => "s.audience IN ('all_staff', 'everyone')",
        };

        let sql = format!(
            r#"SELECT s.id, s.title, s.description, s.audience, s.status,
                      s.is_anonymous, s.expires_at, s.created_at,
                      (SELECT COUNT(*)::bigint FROM survey_questions WHERE survey_id = s.id) AS question_count,
                      (sr.id IS NOT NULL) AS has_responded,
                      sr.submitted_at
               FROM surveys s
               LEFT JOIN survey_responses sr ON sr.survey_id = s.id AND sr.respondent_id = $1
               WHERE s.school_id = $2
                 AND s.status IN ('published', 'expired')
                 AND s.is_template = false
                 AND s.is_anonymous = false
                 AND {}
               ORDER BY
                 CASE WHEN sr.id IS NULL THEN 0 ELSE 1 END,
                 s.created_at DESC"#,
            audience_clause
        );

        let rows = client.query(&sql, &[&user_id, &school_id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|row| MySurveyItem {
            id:             row.get("id"),
            title:          row.get("title"),
            description:    row.get("description"),
            audience:       row.get("audience"),
            status:         row.get("status"),
            is_anonymous:   row.get("is_anonymous"),
            expires_at:     row.get("expires_at"),
            created_at:     row.get("created_at"),
            question_count: row.get("question_count"),
            has_responded:  row.get("has_responded"),
            submitted_at:   row.get("submitted_at"),
        }).collect())
    }

    // ── Submit response ───────────────────────────────────────────────────────

    pub async fn submit_response(
        &self,
        survey_id: Uuid,
        respondent_id: Option<Uuid>,
        school_id: Uuid,
        answers: &[AnswerInput],
    ) -> ApiResult<()> {
        let mut client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let tx = client.transaction().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let response_row = tx.query_one(
            "INSERT INTO survey_responses (survey_id, respondent_id, school_id) \
             VALUES ($1, $2, $3) RETURNING id",
            &[&survey_id, &respondent_id, &school_id],
        ).await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("duplicate key") {
                AppError::Conflict("You have already submitted a response to this survey".to_string())
            } else {
                AppError::Database(msg)
            }
        })?;

        let response_id: Uuid = response_row.get(0);

        for ans in answers {
            let values_json: Option<serde_json::Value> = ans.values.as_ref()
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));
            tx.execute(
                "INSERT INTO survey_response_answers (response_id, question_id, answer_value, answer_values) \
                 VALUES ($1,$2,$3,$4)",
                &[&response_id, &ans.question_id, &ans.value, &values_json],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ── Get aggregated report ─────────────────────────────────────────────────

    pub async fn get_report(&self, survey_id: Uuid) -> ApiResult<SurveyReportDto> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;

        // Get survey info
        let sql = format!("{} WHERE s.id = $1", Self::SURVEY_SELECT);
        let survey_row = client.query_opt(&sql, &[&survey_id])
            .await.map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))?;
        let survey = Self::row_to_list_item(&survey_row);

        // Get all questions
        let q_rows = client.query(
            "SELECT id, question_type, question_text, is_required, options, rating_max, sort_order \
             FROM survey_questions WHERE survey_id = $1 ORDER BY sort_order ASC",
            &[&survey_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        // Get all answers in one query
        let ans_rows = client.query(
            "SELECT sqa.question_id, sqa.answer_value, sqa.answer_values \
             FROM survey_response_answers sqa \
             JOIN survey_responses sr ON sr.id = sqa.response_id \
             WHERE sr.survey_id = $1",
            &[&survey_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        // Build a map: question_id -> vec of (answer_value, answer_values)
        use std::collections::HashMap;
        let mut ans_map: HashMap<Uuid, Vec<(Option<String>, Option<Vec<String>>)>> = HashMap::new();
        for row in &ans_rows {
            let qid: Uuid = row.get("question_id");
            let val: Option<String> = row.get("answer_value");
            let vals: Option<Vec<String>> = row
                .get::<_, Option<serde_json::Value>>("answer_values")
                .and_then(|v| serde_json::from_value(v).ok());
            ans_map.entry(qid).or_default().push((val, vals));
        }

        let mut question_results = Vec::new();
        for q_row in &q_rows {
            let q = Self::row_to_question_dto(q_row);
            let q_id = q.id;
            let q_type = q.question_type.clone();
            let q_text = q.question_text.clone();
            let answers = ans_map.get(&q_id).cloned().unwrap_or_default();
            let total_answered = answers.len() as i64;

            let result = match q_type.as_str() {
                "single_choice" => {
                    let mut counts: HashMap<String, i64> = HashMap::new();
                    for (val, _) in &answers {
                        if let Some(v) = val {
                            if !v.is_empty() {
                                *counts.entry(v.clone()).or_insert(0) += 1;
                            }
                        }
                    }
                    // Preserve option order from question definition
                    let ordered_options = q.options.as_deref().unwrap_or(&[]);
                    let choice_results: Vec<ChoiceResult> = if ordered_options.is_empty() {
                        let mut v: Vec<_> = counts.iter().collect();
                        v.sort_by(|a, b| b.1.cmp(a.1));
                        v.into_iter().map(|(opt, &cnt)| ChoiceResult {
                            option: opt.clone(),
                            count: cnt,
                            percentage: if total_answered > 0 { (cnt as f64 / total_answered as f64) * 100.0 } else { 0.0 },
                        }).collect()
                    } else {
                        ordered_options.iter().map(|opt| {
                            let cnt = *counts.get(opt).unwrap_or(&0);
                            ChoiceResult {
                                option: opt.clone(),
                                count: cnt,
                                percentage: if total_answered > 0 { (cnt as f64 / total_answered as f64) * 100.0 } else { 0.0 },
                            }
                        }).collect()
                    };
                    QuestionResultDto {
                        question_id: q_id,
                        question_text: q_text,
                        question_type: "single_choice".to_string(),
                        total_answered,
                        choice_results: Some(choice_results),
                        average_rating: None,
                        rating_distribution: None,
                        text_responses: None,
                    }
                }
                "multiple_choice" => {
                    let mut counts: HashMap<String, i64> = HashMap::new();
                    for (_, vals) in &answers {
                        if let Some(selected) = vals {
                            for v in selected {
                                *counts.entry(v.clone()).or_insert(0) += 1;
                            }
                        }
                    }
                    let total_resp = total_answered;
                    let ordered_options = q.options.as_deref().unwrap_or(&[]);
                    let choice_results: Vec<ChoiceResult> = if ordered_options.is_empty() {
                        let mut v: Vec<_> = counts.iter().collect();
                        v.sort_by(|a, b| b.1.cmp(a.1));
                        v.into_iter().map(|(opt, &cnt)| ChoiceResult {
                            option: opt.clone(),
                            count: cnt,
                            percentage: if total_resp > 0 { (cnt as f64 / total_resp as f64) * 100.0 } else { 0.0 },
                        }).collect()
                    } else {
                        ordered_options.iter().map(|opt| {
                            let cnt = *counts.get(opt).unwrap_or(&0);
                            ChoiceResult {
                                option: opt.clone(),
                                count: cnt,
                                percentage: if total_resp > 0 { (cnt as f64 / total_resp as f64) * 100.0 } else { 0.0 },
                            }
                        }).collect()
                    };
                    QuestionResultDto {
                        question_id: q_id,
                        question_text: q_text,
                        question_type: "multiple_choice".to_string(),
                        total_answered: total_resp,
                        choice_results: Some(choice_results),
                        average_rating: None,
                        rating_distribution: None,
                        text_responses: None,
                    }
                }
                "rating" => {
                    let rating_max = q.rating_max.unwrap_or(5);
                    let mut star_counts: HashMap<i32, i64> = (1..=rating_max).map(|s| (s, 0)).collect();
                    let mut sum = 0.0f64;
                    let mut valid = 0i64;
                    for (val, _) in &answers {
                        if let Some(v) = val {
                            if let Ok(star) = v.parse::<i32>() {
                                *star_counts.entry(star).or_insert(0) += 1;
                                sum += star as f64;
                                valid += 1;
                            }
                        }
                    }
                    let avg = if valid > 0 { Some(sum / valid as f64) } else { None };
                    let mut dist: Vec<RatingBucket> = (1..=rating_max).map(|star| {
                        let cnt = *star_counts.get(&star).unwrap_or(&0);
                        RatingBucket {
                            star,
                            count: cnt,
                            percentage: if valid > 0 { (cnt as f64 / valid as f64) * 100.0 } else { 0.0 },
                        }
                    }).collect();
                    dist.sort_by(|a, b| b.star.cmp(&a.star)); // descending
                    QuestionResultDto {
                        question_id: q_id,
                        question_text: q_text,
                        question_type: "rating".to_string(),
                        total_answered: valid,
                        choice_results: None,
                        average_rating: avg,
                        rating_distribution: Some(dist),
                        text_responses: None,
                    }
                }
                _ => { // text
                    let texts: Vec<String> = answers.iter()
                        .filter_map(|(val, _)| val.clone())
                        .filter(|v| !v.is_empty())
                        .collect();
                    let total_text = texts.len() as i64;
                    QuestionResultDto {
                        question_id: q_id,
                        question_text: q_text,
                        question_type: "text".to_string(),
                        total_answered: total_text,
                        choice_results: None,
                        average_rating: None,
                        rating_distribution: None,
                        text_responses: Some(texts),
                    }
                }
            };
            question_results.push(result);
        }

        // Individual responses (only for non-anonymous surveys)
        let individual_responses = if !survey.is_anonymous {
            self.get_individual_responses_inner(survey_id, &q_rows, &client).await?
        } else {
            Vec::new()
        };

        Ok(SurveyReportDto { survey, question_results, individual_responses })
    }

    async fn get_individual_responses_inner(
        &self,
        survey_id: Uuid,
        q_rows: &[tokio_postgres::Row],
        client: &tokio_postgres::Client,
    ) -> ApiResult<Vec<IndividualResponseDto>> {
        let resp_rows = client.query(
            r#"SELECT sr.id, sr.respondent_id, sr.submitted_at,
                      u.first_name || ' ' || u.last_name AS respondent_name,
                      u.email AS respondent_email
               FROM survey_responses sr
               LEFT JOIN users u ON u.id = sr.respondent_id
               WHERE sr.survey_id = $1
               ORDER BY sr.submitted_at DESC"#,
            &[&survey_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let all_ans_rows = client.query(
            "SELECT response_id, question_id, answer_value, answer_values \
             FROM survey_response_answers \
             WHERE response_id = ANY(SELECT id FROM survey_responses WHERE survey_id = $1)",
            &[&survey_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        use std::collections::HashMap;
        let mut ans_by_resp: HashMap<Uuid, Vec<&tokio_postgres::Row>> = HashMap::new();
        for row in &all_ans_rows {
            let rid: Uuid = row.get("response_id");
            ans_by_resp.entry(rid).or_default().push(row);
        }

        let q_map: std::collections::HashMap<Uuid, (&str, &str)> = q_rows.iter().map(|r| {
            (r.get::<_, Uuid>("id"), (r.get::<_, &str>("question_type"), r.get::<_, &str>("question_text")))
        }).collect();

        let mut result = Vec::new();
        for resp_row in &resp_rows {
            let resp_id: Uuid = resp_row.get("id");
            let empty: Vec<&tokio_postgres::Row> = Vec::new();
            let ans = ans_by_resp.get(&resp_id).unwrap_or(&empty);

            let answers: Vec<ResponseAnswerDto> = ans.iter().map(|a| {
                let qid: Uuid = a.get("question_id");
                let (q_type, q_text) = q_map.get(&qid).copied().unwrap_or(("text", ""));
                let vals: Option<Vec<String>> = a
                    .get::<_, Option<serde_json::Value>>("answer_values")
                    .and_then(|v| serde_json::from_value(v).ok());
                ResponseAnswerDto {
                    question_id: qid,
                    question_text: q_text.to_string(),
                    question_type: q_type.to_string(),
                    value: a.get("answer_value"),
                    values: vals,
                }
            }).collect();

            result.push(IndividualResponseDto {
                response_id: resp_id,
                respondent_name: resp_row.get("respondent_name"),
                respondent_email: resp_row.get("respondent_email"),
                submitted_at: resp_row.get("submitted_at"),
                answers,
            });
        }
        Ok(result)
    }

    // ── Count targeted users by audience (used at publish time) ───────────────

    pub async fn count_targeted(
        &self,
        school_id: Uuid,
        audience: &str,
        classroom_id: Option<Uuid>,
    ) -> ApiResult<i32> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;

        let count: i64 = match audience {
            "parents" => {
                let row = client.query_one(
                    "SELECT COUNT(DISTINCT u.id)::bigint FROM users u \
                     WHERE u.school_id = $1 AND u.role = 'parent' AND u.is_active = true",
                    &[&school_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;
                row.get(0)
            }
            "employees" => {
                let row = client.query_one(
                    "SELECT COUNT(DISTINCT u.id)::bigint FROM users u \
                     WHERE u.school_id = $1 AND u.role = 'teacher' AND u.is_active = true",
                    &[&school_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;
                row.get(0)
            }
            "all_staff" => {
                let row = client.query_one(
                    "SELECT COUNT(DISTINCT u.id)::bigint FROM users u \
                     WHERE u.school_id = $1 AND u.role IN ('teacher','admin','super_admin') AND u.is_active = true",
                    &[&school_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;
                row.get(0)
            }
            "everyone" => {
                let row = client.query_one(
                    "SELECT COUNT(DISTINCT u.id)::bigint FROM users u \
                     WHERE u.school_id = $1 AND u.is_active = true",
                    &[&school_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;
                row.get(0)
            }
            "classroom" => {
                if let Some(cid) = classroom_id {
                    let row = client.query_one(
                        "SELECT COUNT(DISTINCT u.id)::bigint FROM users u \
                         JOIN children c ON (c.parent_id = u.id OR c.secondary_parent_id = u.id) \
                         JOIN enrollments e ON e.child_id = c.id \
                         WHERE e.classroom_id = $1 AND u.is_active = true",
                        &[&cid],
                    ).await.map_err(|e| AppError::Database(e.to_string()))?;
                    row.get(0)
                } else {
                    0
                }
            }
            _ => 0,
        };
        Ok(count as i32)
    }
}
