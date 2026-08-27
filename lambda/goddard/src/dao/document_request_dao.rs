use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use crate::error::{ApiResult, AppError};
use crate::models::document_request::*;

#[derive(Clone)]
pub struct DocumentRequestDao { pool: Pool }

impl DocumentRequestDao {
    pub fn new(pool: Pool) -> Self { Self { pool } }

    async fn local_now(client: &tokio_postgres::Client, school_id: Uuid) -> ApiResult<chrono::NaiveDateTime> {
        Ok(client.query_one("SELECT school_local_now($1)", &[&school_id]).await
            .map_err(|e| AppError::Database(e.to_string()))?.get(0))
    }

    pub async fn create_request(&self, request: &CreateDocumentRequest, actor_id: Uuid) -> ApiResult<DocumentRequestSummary> {
        let mut client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let transaction = client.transaction().await.map_err(|e| AppError::Database(e.to_string()))?;
        let local_now: chrono::NaiveDateTime = transaction.query_one("SELECT school_local_now($1)", &[&request.school_id]).await.map_err(|e| AppError::Database(e.to_string()))?.get(0);
        let row = transaction.query_one(
            "INSERT INTO document_requests (school_id, audience, target_scope, document_name, instructions, due_date, status, created_by, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,'draft',$7,$8,$8) RETURNING id",
            &[&request.school_id, &request.audience, &request.target, &request.document_name, &request.instructions, &request.due_date, &actor_id, &local_now]
        ).await.map_err(|e| AppError::Database(format!("Failed to create document request: {}", e)))?;
        let request_id: Uuid = row.get(0);
        transaction.execute("INSERT INTO document_audit_events (school_id, document_request_id, event_type, actor_id, created_at) VALUES ($1,$2,'created',$3,$4)", &[&request.school_id, &request_id, &actor_id, &local_now]).await.map_err(|e| AppError::Database(e.to_string()))?;
        transaction.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_request_summary(request_id).await
    }

    pub async fn publish_request(&self, request_id: Uuid, actor_id: Uuid) -> ApiResult<DocumentRequestSummary> {
        let mut client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let transaction = client.transaction().await.map_err(|e| AppError::Database(e.to_string()))?;
        let request = transaction.query_one("SELECT school_id, audience, target_scope, status FROM document_requests WHERE id=$1 FOR UPDATE", &[&request_id]).await.map_err(|e| AppError::Database(e.to_string()))?;
        let school_id: Uuid = request.get(0); let audience: String = request.get(1); let target_scope: String = request.get(2); let status: String = request.get(3);
        if status != "draft" { return Err(AppError::Validation("Only draft document requests can be published".to_string())); }
        let local_now: chrono::NaiveDateTime = transaction.query_one("SELECT school_local_now($1)", &[&school_id]).await.map_err(|e| AppError::Database(e.to_string()))?.get(0);
        if target_scope == "all" && audience == "student" {
            transaction.execute(
                "INSERT INTO document_request_assignments (document_request_id, school_id, enrollment_id, child_id, assigned_at, created_at, updated_at)
                 SELECT $1, e.school_id, e.id, c.id, $2, $2, $2 FROM enrollments e JOIN children c ON c.id=e.child_id
                 WHERE e.school_id=$3 AND e.is_active=true ON CONFLICT (document_request_id, child_id) DO NOTHING",
                &[&request_id, &local_now, &school_id]
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
        } else if target_scope == "all" {
            transaction.execute(
                "INSERT INTO document_request_assignments (document_request_id, school_id, employee_id, assigned_at, created_at, updated_at)
                 SELECT $1, school_id, id, $2, $2, $2 FROM employees WHERE school_id=$3 AND is_active=true
                 ON CONFLICT (document_request_id, employee_id) DO NOTHING",
                &[&request_id, &local_now, &school_id]
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
        }
        transaction.execute("UPDATE document_requests SET status='active', published_at=$2, updated_at=$2 WHERE id=$1", &[&request_id, &local_now]).await.map_err(|e| AppError::Database(e.to_string()))?;
        transaction.execute("INSERT INTO document_audit_events (school_id, document_request_id, event_type, actor_id, created_at) VALUES ($1,$2,'published',$3,$4)", &[&school_id, &request_id, &actor_id, &local_now]).await.map_err(|e| AppError::Database(e.to_string()))?;
        transaction.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        self.get_request_summary(request_id).await
    }

    pub async fn create_selected_assignments(&self, request_id: Uuid, audience: &str, ids: &[Uuid]) -> ApiResult<()> {
        if ids.is_empty() { return Err(AppError::Validation("Select at least one recipient".to_string())); }
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let request = client.query_one("SELECT school_id, status FROM document_requests WHERE id=$1", &[&request_id]).await.map_err(|e| AppError::Database(e.to_string()))?;
        let school_id: Uuid = request.get(0); let status: String = request.get(1);
        if status != "draft" { return Err(AppError::Validation("Recipients can only be selected while the request is a draft".to_string())); }
        let now = Self::local_now(&client, school_id).await?;
        for id in ids {
            if audience == "student" {
                client.execute("INSERT INTO document_request_assignments (document_request_id, school_id, enrollment_id, child_id, assigned_at, created_at, updated_at) SELECT $1,$2,e.id,c.id,$3,$3,$3 FROM children c JOIN enrollments e ON e.child_id=c.id WHERE c.id=$4 AND c.school_id=$2 ON CONFLICT (document_request_id, child_id) DO NOTHING", &[&request_id,&school_id,&now,id]).await.map_err(|e| AppError::Database(e.to_string()))?;
            } else {
                client.execute("INSERT INTO document_request_assignments (document_request_id, school_id, employee_id, assigned_at, created_at, updated_at) SELECT $1,$2,id,$3,$3,$3 FROM employees WHERE id=$4 AND school_id=$2 ON CONFLICT (document_request_id, employee_id) DO NOTHING", &[&request_id,&school_id,&now,id]).await.map_err(|e| AppError::Database(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub async fn list_requests(&self, query: &DocumentRequestQuery) -> ApiResult<Vec<DocumentRequestSummary>> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let rows = client.query(
            "SELECT r.id,r.school_id,r.audience,r.document_name,r.instructions,r.due_date,r.status,r.published_at,
                    COUNT(a.id) FILTER (WHERE a.status='submitted'), COUNT(a.id) FILTER (WHERE a.status='pending'),
                    COUNT(a.id) FILTER (WHERE a.status='approved'), COUNT(a.id) FILTER (WHERE a.status='rejected'), COUNT(a.id)
             FROM document_requests r LEFT JOIN document_request_assignments a ON a.document_request_id=r.id
             WHERE r.school_id=$1 AND ($2::text IS NULL OR r.audience=$2) AND ($3::text IS NULL OR r.status=$3)
               AND ($4::text IS NULL OR r.document_name ILIKE '%' || $4 || '%')
             GROUP BY r.id ORDER BY r.created_at DESC",
            &[&query.school_id, &query.audience, &query.status, &query.search]
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(rows.iter().map(Self::summary).collect())
    }

    pub async fn recipients(&self, school_id: Uuid, audience: &str) -> ApiResult<Vec<DocumentRecipient>> {
        let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;
        let rows=if audience=="student" {
            client.query(
                "SELECT c.id, c.first_name || ' ' || c.last_name, p.email, cl.name
                 FROM enrollments e
                 JOIN children c ON c.id=e.child_id
                 JOIN users p ON p.id=c.parent_id
                 LEFT JOIN classrooms cl ON cl.id=e.classroom_id
                 WHERE e.school_id=$1
                   AND COALESCE(e.is_active,true)=true
                   AND COALESCE(c.is_active,true)=true
                   AND COALESCE(p.is_active,true)=true
                 ORDER BY c.first_name,c.last_name",
                &[&school_id],
            ).await
        } else {
            client.query("SELECT e.id, u.first_name || ' ' || u.last_name, u.email, NULL::text FROM employees e JOIN users u ON u.id=e.user_id WHERE e.school_id=$1 AND COALESCE(e.is_active,true)=true ORDER BY u.first_name,u.last_name", &[&school_id]).await
        }.map_err(|e|AppError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|row|DocumentRecipient{id:row.get(0),name:row.get(1),email:row.get(2),classroom_name:row.get(3)}).collect())
    }

    pub async fn get_request_summary(&self, request_id: Uuid) -> ApiResult<DocumentRequestSummary> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "SELECT r.id,r.school_id,r.audience,r.document_name,r.instructions,r.due_date,r.status,r.published_at,
                    COUNT(a.id) FILTER (WHERE a.status='submitted'), COUNT(a.id) FILTER (WHERE a.status='pending'),
                    COUNT(a.id) FILTER (WHERE a.status='approved'), COUNT(a.id) FILTER (WHERE a.status='rejected'), COUNT(a.id)
             FROM document_requests r LEFT JOIN document_request_assignments a ON a.document_request_id=r.id WHERE r.id=$1 GROUP BY r.id", &[&request_id]
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self::summary(&row))
    }

    fn summary(row: &Row) -> DocumentRequestSummary { DocumentRequestSummary {
        id: row.get(0), school_id: row.get(1), audience: row.get(2), document_name: row.get(3), instructions: row.get(4), due_date: row.get(5), status: row.get(6), published_at: row.get(7), submitted: row.get(8), pending: row.get(9), approved: row.get(10), rejected: row.get(11), total: row.get(12),
    }}

    pub async fn list_assignments(&self, query: &DocumentAssignmentQuery, recipient_user_id: Option<Uuid>, submitted_only: bool) -> ApiResult<PagedDocumentAssignments> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let page=query.page.unwrap_or(1).max(1); let limit=query.limit.unwrap_or(25).clamp(1,100); let offset=(page-1)*limit;
        let extra = if submitted_only { " AND a.status='submitted'" } else { "" };
        let recipient = recipient_user_id.map(|id| id.to_string());
        let sql = format!("SELECT a.id,a.document_request_id,a.school_id,r.audience,r.document_name,r.instructions,r.due_date,r.status,a.status,
              CASE WHEN a.status IN ('pending','rejected') AND r.due_date IS NOT NULL AND r.due_date < school_local_now(a.school_id)::date THEN 'overdue' ELSE a.status END,
              COALESCE(c.first_name || ' ' || c.last_name, u.first_name || ' ' || u.last_name),
              p.first_name || ' ' || p.last_name,p.email,cl.name,u.email,a.submitted_at,a.reviewed_at,a.rejection_reason,a.latest_submission_id,s.original_file_name,s.content_type,s.file_size_bytes,
              (SELECT COUNT(*) FROM document_submissions ds WHERE ds.assignment_id=a.id)
              FROM document_request_assignments a JOIN document_requests r ON r.id=a.document_request_id
              LEFT JOIN children c ON c.id=a.child_id LEFT JOIN enrollments e ON e.id=a.enrollment_id LEFT JOIN users p ON p.id=c.parent_id LEFT JOIN classrooms cl ON cl.id=e.classroom_id
              LEFT JOIN employees emp ON emp.id=a.employee_id LEFT JOIN users u ON u.id=emp.user_id LEFT JOIN document_submissions s ON s.id=a.latest_submission_id
              WHERE a.school_id=$1 AND ($2::text IS NULL OR r.audience=$2) AND ($3::uuid IS NULL OR a.document_request_id=$3) AND ($4::text IS NULL OR a.status=$4) AND ($5::uuid IS NULL OR a.id=$5)
                AND ($6::text IS NULL OR COALESCE(c.first_name || ' ' || c.last_name,u.first_name || ' ' || u.last_name,'') ILIKE '%' || $6 || '%')
                AND ($7::text IS NULL OR (p.id::text=$7 OR c.secondary_parent_id::text=$7 OR u.id::text=$7)){extra}
              ORDER BY COALESCE(a.submitted_at,a.assigned_at) DESC LIMIT $8 OFFSET $9");
        let rows = client.query(&sql, &[&query.school_id,&query.audience,&query.request_id,&query.status,&query.assignment_id,&query.search,&recipient,&limit,&offset]).await.map_err(|e| AppError::Database(e.to_string()))?;
        let count_sql = format!("SELECT COUNT(*) FROM document_request_assignments a JOIN document_requests r ON r.id=a.document_request_id LEFT JOIN children c ON c.id=a.child_id LEFT JOIN employees emp ON emp.id=a.employee_id LEFT JOIN users u ON u.id=emp.user_id LEFT JOIN users p ON p.id=c.parent_id WHERE a.school_id=$1 AND ($2::text IS NULL OR r.audience=$2) AND ($3::uuid IS NULL OR a.document_request_id=$3) AND ($4::text IS NULL OR a.status=$4) AND ($5::text IS NULL OR COALESCE(c.first_name || ' ' || c.last_name,u.first_name || ' ' || u.last_name,'') ILIKE '%' || $5 || '%') AND ($6::text IS NULL OR (p.id::text=$6 OR c.secondary_parent_id::text=$6 OR u.id::text=$6)){extra}");
        let count_sql = format!("SELECT COUNT(*) FROM document_request_assignments a JOIN document_requests r ON r.id=a.document_request_id LEFT JOIN children c ON c.id=a.child_id LEFT JOIN employees emp ON emp.id=a.employee_id LEFT JOIN users u ON u.id=emp.user_id LEFT JOIN users p ON p.id=c.parent_id WHERE a.school_id=$1 AND ($2::text IS NULL OR r.audience=$2) AND ($3::uuid IS NULL OR a.document_request_id=$3) AND ($4::text IS NULL OR a.status=$4) AND ($5::uuid IS NULL OR a.id=$5) AND ($6::text IS NULL OR COALESCE(c.first_name || ' ' || c.last_name,u.first_name || ' ' || u.last_name,'') ILIKE '%' || $6 || '%') AND ($7::text IS NULL OR (p.id::text=$7 OR c.secondary_parent_id::text=$7 OR u.id::text=$7)){extra}");
        let total:i64=client.query_one(&count_sql,&[&query.school_id,&query.audience,&query.request_id,&query.status,&query.assignment_id,&query.search,&recipient]).await.map_err(|e| AppError::Database(e.to_string()))?.get(0);
        Ok(PagedDocumentAssignments { items: rows.iter().map(Self::assignment).collect(), total, page, limit })
    }

    fn assignment(row: &Row) -> DocumentAssignmentItem { DocumentAssignmentItem {
        id:row.get(0),request_id:row.get(1),school_id:row.get(2),audience:row.get(3),document_name:row.get(4),instructions:row.get(5),due_date:row.get(6),request_status:row.get(7),status:row.get(8),derived_status:row.get(9),subject_name:row.get::<_,Option<String>>(10).unwrap_or_else(|| "Unknown recipient".into()),parent_name:row.get(11),parent_email:row.get(12),classroom_name:row.get(13),employee_email:row.get(14),submitted_at:row.get(15),reviewed_at:row.get(16),rejection_reason:row.get(17),latest_submission_id:row.get(18),latest_file_name:row.get(19),latest_content_type:row.get(20),latest_file_size_bytes:row.get(21),version_count:row.get(22),
    }}

    pub async fn assignment_access(&self, assignment_id: Uuid, user_id: Uuid, admin: bool) -> ApiResult<(Uuid,String,String)> {
        let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;
        let row=client.query_opt("SELECT a.school_id,r.audience,r.status FROM document_request_assignments a JOIN document_requests r ON r.id=a.document_request_id LEFT JOIN children c ON c.id=a.child_id LEFT JOIN employees e ON e.id=a.employee_id WHERE a.id=$1 AND ($2 OR c.parent_id=$3 OR c.secondary_parent_id=$3 OR e.user_id=$3)", &[&assignment_id,&admin,&user_id]).await.map_err(|e|AppError::Database(e.to_string()))?.ok_or_else(||AppError::Authorization("You do not have access to this document assignment".to_string()))?;
        Ok((row.get(0),row.get(1),row.get(2)))
    }

    pub async fn recipient_users(&self, request_id: Uuid, assignment_id: Option<Uuid>) -> ApiResult<Vec<Uuid>> {
        let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;
        let rows=client.query("SELECT DISTINCT recipient_id FROM (SELECT c.parent_id AS recipient_id FROM document_request_assignments a JOIN children c ON c.id=a.child_id WHERE a.document_request_id=$1 AND ($2::uuid IS NULL OR a.id=$2) UNION ALL SELECT c.secondary_parent_id FROM document_request_assignments a JOIN children c ON c.id=a.child_id WHERE a.document_request_id=$1 AND ($2::uuid IS NULL OR a.id=$2) UNION ALL SELECT e.user_id FROM document_request_assignments a JOIN employees e ON e.id=a.employee_id WHERE a.document_request_id=$1 AND ($2::uuid IS NULL OR a.id=$2)) recipients WHERE recipient_id IS NOT NULL", &[&request_id,&assignment_id]).await.map_err(|e|AppError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn reminders_for_assignments(&self, school_id: Uuid, assignment_ids: &[Uuid]) -> ApiResult<Vec<DocumentReminder>> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let rows = client.query(
            "SELECT a.id, r.audience, r.document_name, r.due_date, a.rejection_reason,
                    CASE WHEN r.due_date IS NOT NULL AND r.due_date < school_local_now(a.school_id)::date THEN true ELSE false END,
                    COALESCE(c.first_name || ' ' || c.last_name, u.first_name || ' ' || u.last_name), cl.name,
                    p.first_name || ' ' || p.last_name, p.email,
                    sp.first_name || ' ' || sp.last_name, sp.email,
                    u.first_name || ' ' || u.last_name, u.email
             FROM document_request_assignments a
             JOIN document_requests r ON r.id=a.document_request_id
             LEFT JOIN children c ON c.id=a.child_id
             LEFT JOIN enrollments e ON e.id=a.enrollment_id
             LEFT JOIN classrooms cl ON cl.id=e.classroom_id
             LEFT JOIN users p ON p.id=c.parent_id
             LEFT JOIN users sp ON sp.id=c.secondary_parent_id
             LEFT JOIN employees emp ON emp.id=a.employee_id
             LEFT JOIN users u ON u.id=emp.user_id
             WHERE a.school_id=$1 AND a.id = ANY($2) AND a.status IN ('pending','rejected')",
            &[&school_id, &assignment_ids],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let mut reminders = Vec::new();
        for row in rows {
            let assignment_id: Uuid = row.get(0);
            let audience: String = row.get(1);
            let document_name: String = row.get(2);
            let due_date: Option<chrono::NaiveDate> = row.get(3);
            let rejection_reason: Option<String> = row.get(4);
            let is_overdue: bool = row.get(5);
            let subject_name: String = row.get::<_, Option<String>>(6).unwrap_or_else(|| "Recipient".to_string());
            let classroom_name: Option<String> = row.get(7);
            if audience == "student" {
                for (name_index, email_index) in [(8, 9), (10, 11)] {
                    if let Some(email) = row.get::<_, Option<String>>(email_index).filter(|email| !email.trim().is_empty()) {
                        reminders.push(DocumentReminder { assignment_id, audience: audience.clone(), recipient_email: email, recipient_name: row.get::<_, Option<String>>(name_index).unwrap_or_else(|| "Parent".to_string()), subject_name: subject_name.clone(), classroom_name: classroom_name.clone(), document_name: document_name.clone(), due_date, rejection_reason: rejection_reason.clone(), is_overdue });
                    }
                }
            } else if let Some(email) = row.get::<_, Option<String>>(13).filter(|email| !email.trim().is_empty()) {
                reminders.push(DocumentReminder { assignment_id, audience, recipient_email: email, recipient_name: row.get::<_, Option<String>>(12).unwrap_or_else(|| subject_name.clone()), subject_name, classroom_name, document_name, due_date, rejection_reason, is_overdue });
            }
        }
        Ok(reminders)
    }

    pub async fn record_reminders(&self, assignment_ids: &[Uuid], actor_id: Uuid) -> ApiResult<()> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        client.execute(
            "INSERT INTO document_audit_events (school_id, document_request_id, assignment_id, event_type, actor_id, created_at)
             SELECT a.school_id, a.document_request_id, a.id, 'reminder_sent', $2, school_local_now(a.school_id)
             FROM document_request_assignments a WHERE a.id = ANY($1)",
            &[&assignment_ids, &actor_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn complete_upload(&self, assignment_id:Uuid, actor_id:Uuid, data:&CompleteUploadRequest) -> ApiResult<DocumentAssignmentItem> {
        let mut client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;
        let tx=client.transaction().await.map_err(|e|AppError::Database(e.to_string()))?;
        let row=tx.query_one("SELECT school_id,status FROM document_request_assignments WHERE id=$1 FOR UPDATE", &[&assignment_id]).await.map_err(|e|AppError::Database(e.to_string()))?;
        let school_id:Uuid=row.get(0); let status:String=row.get(1);
        if !matches!(status.as_str(), "pending" | "submitted" | "rejected") {
            return Err(AppError::Validation(if status == "approved" { "Approved documents are locked and cannot be replaced".to_string() } else { "This document is not currently available for upload".to_string() }));
        }
        let now:chrono::NaiveDateTime=tx.query_one("SELECT school_local_now($1)",&[&school_id]).await.map_err(|e|AppError::Database(e.to_string()))?.get(0);
        let version:i32=tx.query_one("SELECT COALESCE(MAX(version_number),0)+1 FROM document_submissions WHERE assignment_id=$1",&[&assignment_id]).await.map_err(|e|AppError::Database(e.to_string()))?.get(0);
        let submission=tx.query_one("INSERT INTO document_submissions (assignment_id,school_id,version_number,storage_key,original_file_name,content_type,file_size_bytes,checksum_sha256,uploaded_by,submitted_at,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10) RETURNING id", &[&assignment_id,&school_id,&version,&data.storage_key,&data.file_name,&data.content_type,&data.file_size_bytes,&data.checksum_sha256,&actor_id,&now]).await.map_err(|e|AppError::Database(e.to_string()))?;
        let submission_id:Uuid=submission.get(0);
        tx.execute("UPDATE document_request_assignments SET status='submitted',latest_submission_id=$2,submitted_at=$3,reviewed_at=NULL,reviewed_by=NULL,rejection_reason=NULL,updated_at=$3 WHERE id=$1", &[&assignment_id,&submission_id,&now]).await.map_err(|e|AppError::Database(e.to_string()))?;
        let event_type = match status.as_str() { "submitted" => "replaced", "rejected" => "reuploaded", _ => "uploaded" };
        tx.execute("INSERT INTO document_audit_events (school_id,document_request_id,assignment_id,submission_id,event_type,actor_id,created_at) SELECT school_id,document_request_id,id,$2,$3,$4,$5 FROM document_request_assignments WHERE id=$1", &[&assignment_id,&submission_id,&event_type,&actor_id,&now]).await.map_err(|e|AppError::Database(e.to_string()))?;
        tx.commit().await.map_err(|e|AppError::Database(e.to_string()))?;
        self.assignment_by_id(assignment_id).await
    }

    pub async fn assignment_status(&self, assignment_id: Uuid) -> ApiResult<String> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt("SELECT status FROM document_request_assignments WHERE id = $1", &[&assignment_id]).await.map_err(|e| AppError::Database(e.to_string()))?;
        row.map(|row| row.get(0)).ok_or_else(|| AppError::NotFound("Document assignment not found".into()))
    }

    pub async fn review(&self, assignment_id:Uuid, actor_id:Uuid, review:&ReviewDocumentAssignmentRequest) -> ApiResult<DocumentAssignmentItem> {
        let mut client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?; let tx=client.transaction().await.map_err(|e|AppError::Database(e.to_string()))?;
        let row=tx.query_one("SELECT school_id,document_request_id,status,latest_submission_id FROM document_request_assignments WHERE id=$1 FOR UPDATE", &[&assignment_id]).await.map_err(|e|AppError::Database(e.to_string()))?;
        let school_id:Uuid=row.get(0); let request_id:Uuid=row.get(1); let old:String=row.get(2); let submission:Option<Uuid>=row.get(3);
        if old!="submitted" { return Err(AppError::Validation("Only submitted documents can be reviewed".to_string())); }
        let now:chrono::NaiveDateTime=tx.query_one("SELECT school_local_now($1)",&[&school_id]).await.map_err(|e|AppError::Database(e.to_string()))?.get(0);
        tx.execute("UPDATE document_request_assignments SET status=$2,reviewed_at=$3,reviewed_by=$4,rejection_reason=$5,updated_at=$3 WHERE id=$1", &[&assignment_id,&review.status,&now,&actor_id,&review.reason]).await.map_err(|e|AppError::Database(e.to_string()))?;
        tx.execute("INSERT INTO document_audit_events (school_id,document_request_id,assignment_id,submission_id,event_type,actor_id,reason,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)", &[&school_id,&request_id,&assignment_id,&submission,&review.status,&actor_id,&review.reason,&now]).await.map_err(|e|AppError::Database(e.to_string()))?;
        tx.commit().await.map_err(|e|AppError::Database(e.to_string()))?; self.assignment_by_id(assignment_id).await
    }

    pub async fn assignment_by_id(&self,id:Uuid)->ApiResult<DocumentAssignmentItem>{
        let query=DocumentAssignmentQuery{school_id:Uuid::nil(),audience:None,request_id:None,assignment_id:Some(id),status:None,search:None,page:Some(1),limit:Some(1)};
        let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;
        let school:Uuid=client.query_one("SELECT school_id FROM document_request_assignments WHERE id=$1",&[&id]).await.map_err(|e|AppError::Database(e.to_string()))?.get(0);
        let mut q=query; q.school_id=school;
        let result=self.list_assignments(&q,None,false).await?;
        result.items.into_iter().find(|x|x.id==id).ok_or_else(||AppError::NotFound("Document assignment not found".to_string()))
    }

    pub async fn file_key(&self, submission_id:Uuid)->ApiResult<(Uuid,String)> { let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?; let row=client.query_one("SELECT school_id,storage_key FROM document_submissions WHERE id=$1",&[&submission_id]).await.map_err(|e|AppError::Database(e.to_string()))?; Ok((row.get(0),row.get(1))) }
    pub async fn submission_assignment(&self, submission_id:Uuid)->ApiResult<Uuid>{let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;Ok(client.query_one("SELECT assignment_id FROM document_submissions WHERE id=$1",&[&submission_id]).await.map_err(|e|AppError::Database(e.to_string()))?.get(0))}
    pub async fn history(&self,assignment_id:Uuid)->ApiResult<Vec<DocumentAuditEvent>>{let client=self.pool.get().await.map_err(|e|AppError::Database(e.to_string()))?;let rows=client.query("SELECT a.id,a.event_type,COALESCE(u.first_name || ' ' || u.last_name,u.email),a.reason,a.created_at FROM document_audit_events a LEFT JOIN users u ON u.id=a.actor_id WHERE a.assignment_id=$1 ORDER BY a.created_at DESC",&[&assignment_id]).await.map_err(|e|AppError::Database(e.to_string()))?;Ok(rows.iter().map(|r|DocumentAuditEvent{id:r.get(0),event_type:r.get(1),actor_name:r.get(2),reason:r.get(3),created_at:r.get(4)}).collect())}
}
