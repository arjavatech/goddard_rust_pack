-- =============================================
-- Goddard School Enrollment Management System
-- PRODUCTION Database Schema
-- Exported from Supabase PostgreSQL
-- Date: 2026-02-10
-- =============================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";


-- =============================================
-- TABLES
-- =============================================

-- Table: schools
 CREATE TABLE schools (                          
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     name VARCHAR(255) NOT NULL,                 
     subdomain VARCHAR(100) NOT NULL,            
     settings JSONB,                             
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: users
 CREATE TABLE users (                                        
     id UUID DEFAULT uuid_generate_v4() NOT NULL,            
     school_id UUID NOT NULL,                                
     first_name VARCHAR(100) NOT NULL,                       
     last_name VARCHAR(100) NOT NULL,                        
     email VARCHAR(255) NOT NULL,                            
     role VARCHAR(50) NOT NULL,                              
     is_verified BOOLEAN,                                    
     created_by UUID,                                        
     created_at TIMESTAMP DEFAULT now(),                     
     updated_at TIMESTAMP,                                   
     metadata JSONB,                                         
     is_active BOOLEAN DEFAULT true,                         
     phone_number VARCHAR(20) DEFAULT NULL::character varying
 );


-- Table: children
 CREATE TABLE children (                                    
     id UUID DEFAULT uuid_generate_v4() NOT NULL,           
     parent_id UUID NOT NULL,                               
     secondary_parent_id UUID,                              
     school_id UUID NOT NULL,                               
     first_name VARCHAR(100) NOT NULL,                      
     last_name VARCHAR(100) NOT NULL,                       
     birth_date DATE,                                       
     gender VARCHAR(20),                                    
     status VARCHAR(50) DEFAULT 'active'::character varying,
     is_active BOOLEAN DEFAULT true,                        
     created_at TIMESTAMP DEFAULT now(),                    
     updated_at TIMESTAMP                                   
 );


-- Table: classrooms
 CREATE TABLE classrooms (                       
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     name VARCHAR(255) NOT NULL,                 
     age_group VARCHAR(50),                      
     capacity INTEGER,                           
     enrolled_count INTEGER DEFAULT 0,           
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: enrollments
 CREATE TABLE enrollments (                      
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     child_id UUID NOT NULL,                     
     school_id UUID NOT NULL,                    
     classroom_id UUID NOT NULL,                 
     status VARCHAR(50),                         
     application_status JSONB,                   
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: form_templates
 CREATE TABLE form_templates (                   
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     form_name VARCHAR(255) NOT NULL,            
     form_type VARCHAR(100),                     
     fillout_form_id VARCHAR(255),               
     status VARCHAR(50),                         
     is_required BOOLEAN DEFAULT false,          
     display_order INTEGER,                      
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: student_form_assignments
 CREATE TABLE student_form_assignments (                         
     id UUID DEFAULT uuid_generate_v4() NOT NULL,                
     school_id UUID NOT NULL,                                    
     enrollment_id UUID NOT NULL,                                
     child_id UUID NOT NULL,                                     
     form_template_id UUID NOT NULL,                             
     assignment_source VARCHAR(50),                              
     status VARCHAR(50) DEFAULT 'Not Started'::character varying,
     is_required BOOLEAN DEFAULT false,                          
     assigned_at TIMESTAMP DEFAULT now(),                        
     recent_form_submission_id UUID,                             
     approved_by UUID,                                           
     notes TEXT,                                                 
     approved_on TIMESTAMP,                                      
     is_active BOOLEAN DEFAULT true,                             
     created_at TIMESTAMP DEFAULT now(),                         
     updated_at TIMESTAMP,                                       
     recent_edit_link TEXT,                                      
     recent_pdf_link TEXT                                        
 );


-- Table: form_submissions
 CREATE TABLE form_submissions (                 
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     enrollment_id UUID NOT NULL,                
     student_form_assignment_id UUID NOT NULL,   
     form_template_id UUID NOT NULL,             
     fillout_submission_id VARCHAR(255) NOT NULL,
     form_data JSONB,                            
     metadata JSONB,                             
     submitted_at TIMESTAMP,                     
     processed_at TIMESTAMP,                     
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP,                       
     edit_link TEXT,                             
     pdf_link TEXT                               
 );


-- Table: documents
 CREATE TABLE documents (                        
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     enrollment_id UUID NOT NULL,                
     school_id UUID NOT NULL,                    
     document_type VARCHAR(100),                 
     storage_path TEXT,                          
     file_name VARCHAR(255),                     
     uploaded_at TIMESTAMP DEFAULT now()         
 );


-- Table: class_transitions


-- Table: class_form_overrides
 CREATE TABLE class_form_overrides (             
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     classroom_id UUID NOT NULL,                 
     form_template_id UUID NOT NULL,             
     action VARCHAR(50),                         
     is_required BOOLEAN,                        
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP,                       
     is_active BOOLEAN DEFAULT true              
 );


-- =============================================
-- PRIMARY KEYS
-- =============================================
 ALTER TABLE children ADD CONSTRAINT children_pkey PRIMARY KEY (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_pkey PRIMARY KEY (id);
 ALTER TABLE classrooms ADD CONSTRAINT classrooms_pkey PRIMARY KEY (id);
 ALTER TABLE documents ADD CONSTRAINT documents_pkey PRIMARY KEY (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_pkey PRIMARY KEY (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_pkey PRIMARY KEY (id);
 ALTER TABLE form_templates ADD CONSTRAINT form_templates_pkey PRIMARY KEY (id);
 ALTER TABLE schools ADD CONSTRAINT schools_pkey PRIMARY KEY (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_pkey PRIMARY KEY (id);
 ALTER TABLE users ADD CONSTRAINT users_pkey PRIMARY KEY (id, id);


-- =============================================
-- UNIQUE CONSTRAINTS
-- =============================================
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_fillout_submission_id_key UNIQUE (fillout_submission_id);
 ALTER TABLE schools ADD CONSTRAINT schools_name_key UNIQUE (name);
 ALTER TABLE schools ADD CONSTRAINT schools_subdomain_key UNIQUE (subdomain);
 ALTER TABLE users ADD CONSTRAINT unique_email_per_school UNIQUE (school_id, email);


-- =============================================
-- FOREIGN KEYS
-- =============================================
 ALTER TABLE children ADD CONSTRAINT children_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES users (id);
 ALTER TABLE children ADD CONSTRAINT children_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE children ADD CONSTRAINT children_secondary_parent_id_fkey FOREIGN KEY (secondary_parent_id) REFERENCES users (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_classroom_id_fkey FOREIGN KEY (classroom_id) REFERENCES classrooms (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_form_template_id_fkey FOREIGN KEY (form_template_id) REFERENCES form_templates (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE classrooms ADD CONSTRAINT classrooms_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE documents ADD CONSTRAINT documents_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE documents ADD CONSTRAINT documents_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_child_id_fkey FOREIGN KEY (child_id) REFERENCES children (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_classroom_id_fkey FOREIGN KEY (classroom_id) REFERENCES classrooms (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_form_template_id_fkey FOREIGN KEY (form_template_id) REFERENCES form_templates (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_student_form_assignment_id_fkey FOREIGN KEY (student_form_assignment_id) REFERENCES student_form_assignments (id);
 ALTER TABLE form_templates ADD CONSTRAINT form_templates_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_child_id_fkey FOREIGN KEY (child_id) REFERENCES children (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_form_template_id_fkey FOREIGN KEY (form_template_id) REFERENCES form_templates (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE users ADD CONSTRAINT users_created_by_fkey FOREIGN KEY (created_by) REFERENCES users (id);
 ALTER TABLE users ADD CONSTRAINT users_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);


-- =============================================
-- FUNCTIONS
-- =============================================
                                                                                                                             
 -- Function: check_enrollment_completion                                                                                    
 CREATE OR REPLACE FUNCTION public.check_enrollment_completion(p_enrollment_id uuid)                                         
  RETURNS void                                                                                                               
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     total_forms INTEGER;                                                                                                    
     approved_forms INTEGER;                                                                                                 
     all_approved BOOLEAN;                                                                                                   
 BEGIN                                                                                                                       
     -- Count total active form assignments for this enrollment                                                              
     SELECT COUNT(*)                                                                                                         
     INTO total_forms                                                                                                        
     FROM student_form_assignments                                                                                           
     WHERE enrollment_id = p_enrollment_id                                                                                   
       AND is_active = true;                                                                                                 
                                                                                                                             
     -- Count how many forms are approved                                                                                    
     -- You can adjust the status check based on your business logic                                                         
     -- Options: 'approved', 'completed', or both                                                                            
     SELECT COUNT(*)                                                                                                         
     INTO approved_forms                                                                                                     
     FROM student_form_assignments                                                                                           
     WHERE enrollment_id = p_enrollment_id                                                                                   
       AND is_active = true                                                                                                  
       AND status IN ('approved', 'completed');  -- Adjust as needed                                                         
                                                                                                                             
     -- Determine if all forms are approved                                                                                  
     all_approved := (total_forms > 0 AND total_forms = approved_forms);                                                     
                                                                                                                             
     -- Update enrollment status based on completion                                                                         
     IF all_approved THEN                                                                                                    
         UPDATE enrollments                                                                                                  
         SET                                                                                                                 
             status = 'completed',                                                                                           
             updated_at = NOW()                                                                                              
         WHERE id = p_enrollment_id                                                                                          
           AND status != 'completed';  -- Only update if not already completed                                               
     ELSE                                                                                                                    
         -- Optional: Set back to 'incomplete' or 'pending' if not all approved                                              
         -- Uncomment the following if you want to auto-revert status                                                        
         /*                                                                                                                  
         UPDATE enrollments                                                                                                  
         SET                                                                                                                 
             status = 'incomplete',                                                                                          
             updated_at = NOW()                                                                                              
         WHERE id = p_enrollment_id                                                                                          
           AND status = 'completed';  -- Only downgrade if it was completed                                                  
         */                                                                                                                  
     END IF;                                                                                                                 
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: handle_new_auth_user                                                                                           
 CREATE OR REPLACE FUNCTION public.handle_new_auth_user()                                                                    
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
  SECURITY DEFINER                                                                                                           
 AS $function$                                                                                                               
 BEGIN                                                                                                                       
     -- Insert new user into public.users table using auth user data                                                         
     INSERT INTO public.users (                                                                                              
         id,                                                                                                                 
         school_id,                                                                                                          
         first_name,                                                                                                         
         last_name,                                                                                                          
         email,                                                                                                              
         role,                                                                                                               
         is_verified,                                                                                                        
         created_at,                                                                                                         
         metadata,                                                                                                           
         is_active                                                                                                           
     ) VALUES (                                                                                                              
         NEW.id,                                                                                                             
         COALESCE(                                                                                                           
             (NEW.raw_user_meta_data->>'school_id')::UUID,                                                                   
             NULL                                                                                                            
         ),                                                                                                                  
         COALESCE(                                                                                                           
             NEW.raw_user_meta_data->>'first_name',                                                                          
             ''                                                                                                              
         ),                                                                                                                  
         COALESCE(                                                                                                           
             NEW.raw_user_meta_data->>'last_name',                                                                           
             ''                                                                                                              
         ),                                                                                                                  
         NEW.email,                                                                                                          
         COALESCE(                                                                                                           
             NEW.raw_user_meta_data->>'role',                                                                                
             'Parent'                                                                                                        
         ),                                                                                                                  
         -- Respect is_verified from metadata if provided, otherwise default based on role                                   
         COALESCE(                                                                                                           
             (NEW.raw_user_meta_data->>'is_verified')::boolean,                                                              
             CASE                                                                                                            
                 WHEN COALESCE(NEW.raw_user_meta_data->>'role', 'Parent') IN ('Parent', 'primary-parent', 'secondary-parent')
                 THEN true                                                                                                   
                 ELSE false                                                                                                  
             END                                                                                                             
         ),                                                                                                                  
         NOW(),                                                                                                              
         NEW.raw_user_meta_data,                                                                                             
         true                                                                                                                
     );                                                                                                                      
                                                                                                                             
     RETURN NEW;                                                                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: set_is_verified_based_on_role                                                                                  
 CREATE OR REPLACE FUNCTION public.set_is_verified_based_on_role()                                                           
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 BEGIN                                                                                                                       
     -- Set is_verified based on role                                                                                        
     IF NEW.is_verified IS NULL THEN                                                                                         
         IF NEW.role = 'Parent' OR NEW.role = 'primary-parent' OR NEW.role = 'secondary-parent' THEN                         
             NEW.is_verified := true;                                                                                        
         ELSE                                                                                                                
             NEW.is_verified := false;                                                                                       
         END IF;                                                                                                             
     END IF;                                                                                                                 
     RETURN NEW;                                                                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: sync_all_enrollment_statuses                                                                                   
 CREATE OR REPLACE FUNCTION public.sync_all_enrollment_statuses()                                                            
  RETURNS void                                                                                                               
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     enrollment_record RECORD;                                                                                               
 BEGIN                                                                                                                       
     -- Loop through all enrollments and sync their statuses                                                                 
     FOR enrollment_record IN                                                                                                
         SELECT DISTINCT id FROM enrollments WHERE is_active = true                                                          
     LOOP                                                                                                                    
         -- Build and update application_status for each enrollment                                                          
         UPDATE enrollments e                                                                                                
         SET application_status = (                                                                                          
             SELECT COALESCE(                                                                                                
                 jsonb_object_agg(ft.form_name, sfa.status),                                                                 
                 '{}'::jsonb                                                                                                 
             )                                                                                                               
             FROM student_form_assignments sfa                                                                               
             JOIN form_templates ft ON ft.id = sfa.form_template_id                                                          
             WHERE sfa.enrollment_id = enrollment_record.id                                                                  
               AND sfa.is_active = true                                                                                      
         ),                                                                                                                  
         updated_at = NOW()                                                                                                  
         WHERE e.id = enrollment_record.id;                                                                                  
                                                                                                                             
         -- Check completion status                                                                                          
         PERFORM check_enrollment_completion(enrollment_record.id);                                                          
     END LOOP;                                                                                                               
                                                                                                                             
     RAISE NOTICE 'Synced all enrollment statuses successfully';                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: sync_enrollment_form_status                                                                                    
 CREATE OR REPLACE FUNCTION public.sync_enrollment_form_status()                                                             
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     target_enrollment_id UUID;                                                                                              
     new_application_status JSONB;                                                                                           
 BEGIN                                                                                                                       
     -- Determine which enrollment_id to update                                                                              
     -- Handle INSERT/UPDATE (use NEW) and DELETE (use OLD)                                                                  
     IF TG_OP = 'DELETE' THEN                                                                                                
         target_enrollment_id := OLD.enrollment_id;                                                                          
     ELSE                                                                                                                    
         target_enrollment_id := NEW.enrollment_id;                                                                          
     END IF;                                                                                                                 
                                                                                                                             
     -- Build the application_status JSONB by aggregating all form assignments                                               
     -- Join with form_templates to get the form_name                                                                        
     SELECT COALESCE(                                                                                                        
         jsonb_object_agg(                                                                                                   
             ft.form_name,  -- Key: form name from form_templates                                                            
             sfa.status     -- Value: status from student_form_assignments                                                   
         ),                                                                                                                  
         '{}'::jsonb  -- Empty JSONB if no forms found                                                                       
     )                                                                                                                       
     INTO new_application_status                                                                                             
     FROM student_form_assignments sfa                                                                                       
     JOIN form_templates ft ON ft.id = sfa.form_template_id                                                                  
     WHERE sfa.enrollment_id = target_enrollment_id                                                                          
       AND sfa.is_active = true;  -- Only include active assignments                                                         
                                                                                                                             
     -- Update the enrollments table with the new application_status                                                         
     UPDATE enrollments                                                                                                      
     SET                                                                                                                     
         application_status = new_application_status,                                                                        
         updated_at = NOW()                                                                                                  
     WHERE id = target_enrollment_id;                                                                                        
                                                                                                                             
     -- After updating application_status, check if enrollment should be completed                                           
     PERFORM check_enrollment_completion(target_enrollment_id);                                                              
                                                                                                                             
     -- Return appropriate value based on operation                                                                          
     IF TG_OP = 'DELETE' THEN                                                                                                
         RETURN OLD;                                                                                                         
     ELSE                                                                                                                    
         RETURN NEW;                                                                                                         
     END IF;                                                                                                                 
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: sync_existing_auth_users                                                                                       
 CREATE OR REPLACE FUNCTION public.sync_existing_auth_users()                                                                
  RETURNS void                                                                                                               
  LANGUAGE plpgsql                                                                                                           
  SECURITY DEFINER                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     auth_user RECORD;                                                                                                       
 BEGIN                                                                                                                       
     FOR auth_user IN                                                                                                        
         SELECT * FROM auth.users                                                                                            
         WHERE id NOT IN (SELECT id FROM public.users)                                                                       
     LOOP                                                                                                                    
         INSERT INTO public.users (                                                                                          
             id,                                                                                                             
             school_id,                                                                                                      
             first_name,                                                                                                     
             last_name,                                                                                                      
             email,                                                                                                          
             role,                                                                                                           
             is_verified,                                                                                                    
             created_at,                                                                                                     
             metadata,                                                                                                       
             is_active                                                                                                       
         ) VALUES (                                                                                                          
             auth_user.id,                                                                                                   
             COALESCE(                                                                                                       
                 (auth_user.raw_user_meta_data->>'school_id')::UUID,                                                         
                 NULL                                                                                                        
             ),                                                                                                              
             COALESCE(                                                                                                       
                 auth_user.raw_user_meta_data->>'first_name',                                                                
                 ''                                                                                                          
             ),                                                                                                              
             COALESCE(                                                                                                       
                 auth_user.raw_user_meta_data->>'last_name',                                                                 
                 ''                                                                                                          
             ),                                                                                                              
             auth_user.email,                                                                                                
             COALESCE(                                                                                                       
                 auth_user.raw_user_meta_data->>'role',                                                                      
                 'Parent'                                                                                                    
             ),                                                                                                              
             COALESCE(auth_user.email_confirmed_at IS NOT NULL, false),                                                      
             auth_user.created_at,                                                                                           
             auth_user.raw_user_meta_data,                                                                                   
             true                                                                                                            
         ) ON CONFLICT (id) DO NOTHING;                                                                                      
     END LOOP;                                                                                                               
 END;                                                                                                                        
 $function$                                                                                                                  
 


-- =============================================
-- TRIGGERS
-- =============================================
                                                                                                                                                         
 -- Trigger on table: student_form_assignments                                                                                                           
 CREATE TRIGGER trigger_sync_enrollment_form_status AFTER INSERT ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION sync_enrollment_form_status();
                                                                                                                                                         
 -- Trigger on table: student_form_assignments                                                                                                           
 CREATE TRIGGER trigger_sync_enrollment_form_status AFTER DELETE ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION sync_enrollment_form_status();
                                                                                                                                                         
 -- Trigger on table: student_form_assignments                                                                                                           
 CREATE TRIGGER trigger_sync_enrollment_form_status AFTER UPDATE ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION sync_enrollment_form_status();
                                                                                                                                                         
 -- Trigger on table: users                                                                                                                              
 CREATE TRIGGER set_is_verified_on_insert BEFORE INSERT ON users FOR EACH ROW EXECUTE FUNCTION set_is_verified_based_on_role();


-- =============================================
-- DATA DUMP
-- =============================================

-- Data for table: schools
-- Rows:      1
\COPY schools FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,name,subdomain,settings,is_active,created_at,updated_at
9276e4d9-7c52-4710-99d6-80a3e0bccab2,"Goddard School, Lynnwood",lynnwood,"{""timezone"": ""America/New_York"", ""age_groups"": [""infants"", ""toddlers"", ""preschool"", ""pre-k""], ""enrollment_capacity"": 200}",t,2025-10-06 08:55:32.685604,
\.

-- Data for table: users
-- Rows:     13
\COPY users FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,school_id,first_name,last_name,email,role,is_verified,created_by,created_at,updated_at,metadata,is_active,phone_number
6eae1b0e-d835-4590-871d-e781f798f3e1,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Goddard,Admin,goddardschool01@gmail.com,Admin,t,,2025-10-28 10:25:03.76934,,"{""role"": ""Admin"", ""last_name"": ""Admin"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Goddard""}",t,
45352fb3-358f-4c40-bdd5-84ffa236e800,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Test,Parent,test-parent-1761650065@example.com,Parent,t,,2025-10-28 11:14:27.078566,,"{""role"": ""Parent"", ""last_name"": ""Parent"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Test""}",t,
1d672ebb-03d7-4431-a52e-6f13542ebc9f,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Arun,Kumar,arunkumar.arjava@gmail.com,Parent,t,,2025-10-28 11:14:58.632884,2025-10-28 17:41:31.747954,"{""role"": ""Parent"", ""last_name"": ""Kumar"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Arun""}",f,
072cdb2a-5ef1-40af-a476-455e38d2ecc3,9276e4d9-7c52-4710-99d6-80a3e0bccab2,logi,A,logeshwari.arjava@gmail.com,Parent,t,,2025-10-28 11:17:29.302197,2025-10-28 17:41:38.035493,"{""role"": ""Parent"", ""last_name"": ""A"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""logi""}",f,
005a4691-71b5-469e-b39f-d1ed74450214,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Mani,RR,mani.arjava@gmail.com,Parent,t,,2025-10-28 13:26:37.279976,2025-10-28 17:41:44.002258,"{""role"": ""Parent"", ""last_name"": ""RR"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Mani""}",f,
dc9fc7bf-8a8a-4aed-87b7-33cd70b1cc3c,9276e4d9-7c52-4710-99d6-80a3e0bccab2,vaishnavi,sara,vaishnavi.arjava@gmail.com,Parent,t,,2025-10-28 11:27:19.293318,2025-10-28 17:41:50.572631,"{""role"": ""Parent"", ""last_name"": ""sara"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""vaishnavi""}",f,
ba6fa215-6b9a-4210-95af-4119915f241f,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Mani,RRP,pitchumaniece+1@gmail.com,Parent,t,,2025-11-11 05:37:04.970761,2025-11-13 03:02:15.9256,"{""role"": ""Parent"", ""last_name"": ""RRP"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Mani""}",f,
2c164dca-70bd-4608-87a2-be55fb244204,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Mani,RRP,pitchumaniece+135@gmail.com,Parent,t,,2025-11-11 05:31:09.073509,2025-11-11 05:31:42.944245,"{""role"": ""Parent"", ""last_name"": ""RRP"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Mani""}",f,
466eb479-f16b-4d22-b68e-c5e8c1b3387a,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Paul,McCoard,paulbmcc@gmail.com,Parent,t,,2026-02-07 16:39:16.075189,,"{""role"": ""Parent"", ""last_name"": ""McCoard"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""Paul"", ""is_verified"": null, ""phone_number"": null}",t,
c2913433-3186-40b7-ab29-8e16e7426261,9276e4d9-7c52-4710-99d6-80a3e0bccab2,kalai,S,kalai.arjava@gmail.com,Parent,t,,2025-11-11 05:13:26.332674,2025-11-13 03:02:07.241217,"{""role"": ""Parent"", ""last_name"": ""S"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""kalai""}",f,
f036cb35-85c3-489c-8d61-a6565f28aaf0,9276e4d9-7c52-4710-99d6-80a3e0bccab2,karthi,raj,karthi.arjava@gmail.com,Parent,t,,2025-11-11 10:03:08.790083,2025-11-13 03:02:11.641651,"{""role"": ""Parent"", ""last_name"": ""raj"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""karthi""}",f,
e1c34b1a-db04-49e7-b411-f1fa1adc5ca7,9276e4d9-7c52-4710-99d6-80a3e0bccab2,mickel,johnas,vanikalai.moorthy@gmail.com,Parent,t,,2026-02-08 09:08:55.723541,,"{""role"": ""Parent"", ""last_name"": ""johnas"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""mickel"", ""is_verified"": null, ""phone_number"": null}",t,
5b2cf0e9-11b0-4d42-b9e5-3e74d057ec58,9276e4d9-7c52-4710-99d6-80a3e0bccab2,maya,johnas,vanikalai.moorthy7@gmail.com,secondary-parent,t,,2026-02-08 09:08:57.158105,,"{""role"": ""secondary-parent"", ""last_name"": ""johnas"", ""school_id"": ""9276e4d9-7c52-4710-99d6-80a3e0bccab2"", ""first_name"": ""maya"", ""is_verified"": null, ""phone_number"": null}",t,
\.

-- Data for table: children
-- Rows:     15
\COPY children FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,parent_id,secondary_parent_id,school_id,first_name,last_name,birth_date,gender,status,is_active,created_at,updated_at
71d28ae1-da03-4a99-9b92-a24bae7e42e7,1d672ebb-03d7-4431-a52e-6f13542ebc9f,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Sakthi,Arun,2016-06-07,male,active,f,2025-10-28 11:15:00.111449,2025-10-28 17:41:31.747954
37f5a98e-b5a9-422c-b92c-27403f2d008b,072cdb2a-5ef1-40af-a476-455e38d2ecc3,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,kiran,K,2020-12-02,male,active,f,2025-10-28 11:17:30.828105,2025-10-28 17:41:38.035493
947f2f4c-df83-4eef-a7bc-836431bdd0e2,072cdb2a-5ef1-40af-a476-455e38d2ecc3,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Regina,Logi,2020-12-16,female,active,f,2025-10-28 12:08:25.799863,2025-10-28 17:41:38.035493
850930b4-3486-4d8e-9635-168b61d230d0,072cdb2a-5ef1-40af-a476-455e38d2ecc3,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Reena,k,2005-02-28,female,active,f,2025-10-28 14:25:19.348357,2025-10-28 17:41:38.035493
b5c71b98-b57e-4c53-b9d0-92a254fccd50,005a4691-71b5-469e-b39f-d1ed74450214,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Ram,RR,2020-12-03,male,active,f,2025-10-28 13:26:38.878484,2025-10-28 17:41:44.002258
4380f2f7-f8dc-46a8-b8fb-b5a05de9abbb,dc9fc7bf-8a8a-4aed-87b7-33cd70b1cc3c,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,yash,Ari,2020-12-16,male,active,f,2025-10-28 11:27:20.845034,2025-10-28 17:41:50.572631
629679fd-482d-4104-beca-183427b35b0e,dc9fc7bf-8a8a-4aed-87b7-33cd70b1cc3c,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,tara,navya,2020-12-03,female,active,f,2025-10-28 11:32:33.471851,2025-10-28 17:41:50.572631
6475f4a0-88fc-4d10-9352-e5d97910c83e,dc9fc7bf-8a8a-4aed-87b7-33cd70b1cc3c,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,navya,sa,2020-12-09,female,active,f,2025-10-28 11:59:36.23508,2025-10-28 17:41:50.572631
67a90ee3-aaee-4c1d-9836-453eb9dc6a92,2c164dca-70bd-4608-87a2-be55fb244204,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Arun,Paiyan,2018-05-15,male,active,f,2025-11-11 05:31:10.553471,2025-11-11 05:31:42.944245
98d9e63c-f3cf-4dda-9383-d47b0d72385b,c2913433-3186-40b7-ab29-8e16e7426261,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,mike,jsd,2020-12-17,male,active,f,2025-11-11 05:13:28.340393,2025-11-13 03:02:07.241217
48a8848e-c984-431d-8827-91078ca8d679,c2913433-3186-40b7-ab29-8e16e7426261,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,nancy,wheeler,2020-12-17,female,active,f,2025-11-11 14:35:12.821365,2025-11-13 03:02:07.241217
1be77d9d-7c92-4a0f-af21-b1639ddc983b,f036cb35-85c3-489c-8d61-a6565f28aaf0,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,karthi,raj,2020-12-10,male,active,f,2025-11-11 10:03:10.237413,2025-11-13 03:02:11.641651
5dff0e9d-97af-4945-bb2b-8d69166a17fa,ba6fa215-6b9a-4210-95af-4119915f241f,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Arun,RR,2018-05-15,male,active,f,2025-11-11 05:37:06.381757,2025-11-13 03:02:15.9256
bcfc9031-2902-4fac-9c1c-48aad08624ee,466eb479-f16b-4d22-b68e-c5e8c1b3387a,,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Michael,McCoard,2020-12-20,male,active,t,2026-02-07 16:39:17.310994,2026-02-07 16:39:17.310994
31870db7-1ccb-42ee-8227-f16a3a2a894b,e1c34b1a-db04-49e7-b411-f1fa1adc5ca7,5b2cf0e9-11b0-4d42-b9e5-3e74d057ec58,9276e4d9-7c52-4710-99d6-80a3e0bccab2,mike,johnas,2020-12-30,male,active,t,2026-02-08 09:08:58.210705,2026-02-08 09:08:58.210705
\.

-- Data for table: classrooms
-- Rows:      4
\COPY classrooms FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,school_id,name,age_group,capacity,enrolled_count,is_active,created_at,updated_at
193a6138-105c-4192-b3af-c9e7472b4542,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Class-B,,,0,t,2025-10-28 11:32:51.86945,2025-10-28 11:32:51.86945
3171f61b-61a8-4dc9-942a-5cb30fcea621,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Class-C,,,0,t,2025-10-28 11:33:07.25922,2025-10-28 11:33:07.25922
5d265168-72d0-4304-9c84-973d969a6557,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Class-D,,,0,f,2025-11-11 05:23:20.055668,2025-11-11 09:56:10.061491
26464a03-3824-4b9c-8b38-fd1f7e5b6deb,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Herons,,,0,t,2025-10-28 10:46:09.800972,2026-02-07 16:32:59.146558
\.

-- Data for table: enrollments
-- Rows:     15
\COPY enrollments FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,child_id,school_id,classroom_id,status,application_status,is_active,created_at,updated_at
5bc1df49-767c-427c-b750-9b60c097bbdf,947f2f4c-df83-4eef-a7bc-836431bdd0e2,9276e4d9-7c52-4710-99d6-80a3e0bccab2,193a6138-105c-4192-b3af-c9e7472b4542,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""in_progress""}",f,2025-10-28 12:08:26.100442,2025-10-28 17:41:38.035493
274ebe13-cf0a-48c7-8da0-d9b16f54aa6b,37f5a98e-b5a9-422c-b92c-27403f2d008b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-10-28 11:17:31.116875,2025-10-28 17:41:38.035493
41143921-eb7b-4f8d-b529-8f1f950cd094,850930b4-3486-4d8e-9635-168b61d230d0,9276e4d9-7c52-4710-99d6-80a3e0bccab2,193a6138-105c-4192-b3af-c9e7472b4542,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""in_progress"", ""Parent Handbook"": ""in_progress"", ""Authorization Form"": ""in_progress""}",f,2025-10-28 14:25:19.648999,2025-10-28 17:41:38.035493
1cd2966f-5497-4470-9570-48923c6fbc83,b5c71b98-b57e-4c53-b9d0-92a254fccd50,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""in_progress""}",f,2025-10-28 13:26:39.181576,2025-10-28 17:41:44.002258
e11ab8a0-f041-4e25-ac38-77f87797c5f2,4380f2f7-f8dc-46a8-b8fb-b5a05de9abbb,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-10-28 11:27:21.178319,2025-10-28 17:41:50.572631
2d67758d-0ff9-4098-85f2-333b9211d601,629679fd-482d-4104-beca-183427b35b0e,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-10-28 11:32:33.762623,2025-10-28 17:41:50.572631
2a3fd199-9e5b-412d-9359-c4215559afec,6475f4a0-88fc-4d10-9352-e5d97910c83e,9276e4d9-7c52-4710-99d6-80a3e0bccab2,193a6138-105c-4192-b3af-c9e7472b4542,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-10-28 11:59:36.527103,2025-10-28 17:41:50.572631
f4d0b93e-4c29-4b95-83b9-a445add82224,bcfc9031-2902-4fac-9c1c-48aad08624ee,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",t,2026-02-07 16:39:17.619234,2026-02-07 16:39:17.808288
eb76631f-c9c9-4f6d-83e3-b0c35a52f8eb,31870db7-1ccb-42ee-8227-f16a3a2a894b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,3171f61b-61a8-4dc9-942a-5cb30fcea621,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",t,2026-02-08 09:08:58.6118,2026-02-08 09:08:58.810576
a8bedec4-2988-4021-b9db-6f85f575601a,48a8848e-c984-431d-8827-91078ca8d679,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""rejected"", ""Authorization Form"": ""in_progress""}",f,2025-11-11 14:35:13.233502,2025-11-13 03:02:07.241217
f0015897-3515-44bf-9f3c-19c113749066,98d9e63c-f3cf-4dda-9383-d47b0d72385b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""approved"", ""Authorization Form"": ""approved""}",f,2025-11-11 05:13:28.675393,2025-11-13 03:02:07.241217
c94d23d7-45ae-4521-8544-0546bb90087a,1be77d9d-7c92-4a0f-af21-b1639ddc983b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-11-11 10:03:10.552371,2025-11-13 03:02:11.641651
896a8bf8-261b-441e-9547-b7647b269ee6,5dff0e9d-97af-4945-bb2b-8d69166a17fa,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-11-11 05:37:06.693826,2025-11-13 03:02:15.9256
b2097795-5db7-4197-a94a-096030ba8c6b,71d28ae1-da03-4a99-9b92-a24bae7e42e7,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-10-28 11:15:00.40508,2025-10-28 17:41:31.747954
2b674756-a7dc-44ba-a242-c77653c805d8,67a90ee3-aaee-4c1d-9836-453eb9dc6a92,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,incomplete,"{""Admission Form"": ""incomplete"", ""Enrollment Form"": ""incomplete"", ""Parent Handbook"": ""incomplete"", ""Authorization Form"": ""incomplete""}",f,2025-11-11 05:31:10.869249,2025-11-11 05:31:42.944245
\.

-- Data for table: form_templates
-- Rows:      6
\COPY form_templates FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,school_id,form_name,form_type,fillout_form_id,status,is_required,display_order,is_active,created_at,updated_at
b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Authorization Form,,https://goddard.fillout.com/t/uxYBSkibvFus?student_form_assignment_id=xxxxx,school_default,,,t,2025-10-28 10:32:06.37779,2025-10-28 12:04:05.226143
0b9ccb32-19cc-4634-82d2-9d6e81ea527e,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Enrollment Form,,https://goddard.fillout.com/t/gXRdaCT2rKus?student_form_assignment_id=xxxxx,school_default,,,t,2025-10-28 10:42:09.611201,2025-10-28 12:04:54.803889
35f76c68-eb06-47c3-8888-a4230bdafd99,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Parent Handbook,,https://goddard.fillout.com/t/mNy14Tpfu1us?student_form_assignment_id=xxxxx,school_default,,,t,2025-10-28 10:57:54.607395,2025-10-28 12:05:58.201602
7eb674c0-0bcf-48d4-8f0f-745fd29feaee,9276e4d9-7c52-4710-99d6-80a3e0bccab2,Admission Form,,https://goddard.fillout.com/t/2T7C5onHgcus?student_form_assignment_id=xxxxx,school_default,,,t,2025-10-28 10:44:27.650824,2025-10-28 12:06:45.325697
b2dc0335-54b3-44d6-9f1e-a5a872b64725,9276e4d9-7c52-4710-99d6-80a3e0bccab2,df,,https://fd.coms,school_default,,,f,2025-11-11 09:59:30.518415,2025-11-11 09:59:59.091373
113bef5a-9758-41f0-97e8-92783ee7bfdb,9276e4d9-7c52-4710-99d6-80a3e0bccab2,er,,re,school_default,f,,f,2025-11-11 10:01:32.15164,2025-11-11 10:01:44.656085
\.

-- Data for table: student_form_assignments
-- Rows:     60
\COPY student_form_assignments FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,school_id,enrollment_id,child_id,form_template_id,assignment_source,status,is_required,assigned_at,recent_form_submission_id,approved_by,notes,approved_on,is_active,created_at,updated_at,recent_edit_link,recent_pdf_link
5b1fbb96-d6c2-4777-aefd-264fc22973bf,9276e4d9-7c52-4710-99d6-80a3e0bccab2,b2097795-5db7-4197-a94a-096030ba8c6b,71d28ae1-da03-4a99-9b92-a24bae7e42e7,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-10-28 11:15:00.781162,,,,,t,2025-10-28 11:15:00.781162,,,
fd33e7fb-94b9-4d54-80cd-5ed23eb9920d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,b2097795-5db7-4197-a94a-096030ba8c6b,71d28ae1-da03-4a99-9b92-a24bae7e42e7,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 11:15:00.781162,,,,,t,2025-10-28 11:15:00.781162,,,
18794e84-91f9-41fc-8677-b9961b170802,9276e4d9-7c52-4710-99d6-80a3e0bccab2,b2097795-5db7-4197-a94a-096030ba8c6b,71d28ae1-da03-4a99-9b92-a24bae7e42e7,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 11:15:00.781162,,,,,t,2025-10-28 11:15:00.781162,,,
79da38be-8919-44b7-a576-f44f57a1af43,9276e4d9-7c52-4710-99d6-80a3e0bccab2,b2097795-5db7-4197-a94a-096030ba8c6b,71d28ae1-da03-4a99-9b92-a24bae7e42e7,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 11:15:00.781162,,,,,t,2025-10-28 11:15:00.781162,,,
5635347a-18c8-4d74-a0a3-a4bc5139fdb5,9276e4d9-7c52-4710-99d6-80a3e0bccab2,274ebe13-cf0a-48c7-8da0-d9b16f54aa6b,37f5a98e-b5a9-422c-b92c-27403f2d008b,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 11:17:31.28229,,,,,t,2025-10-28 11:17:31.28229,,,
6c9db51d-4f94-4d9c-9ea8-25c21bea0cd8,9276e4d9-7c52-4710-99d6-80a3e0bccab2,274ebe13-cf0a-48c7-8da0-d9b16f54aa6b,37f5a98e-b5a9-422c-b92c-27403f2d008b,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 11:17:31.28229,,,,,t,2025-10-28 11:17:31.28229,,,
c9e1ad70-0dcb-47b9-8dad-93e6d9d7fdf4,9276e4d9-7c52-4710-99d6-80a3e0bccab2,274ebe13-cf0a-48c7-8da0-d9b16f54aa6b,37f5a98e-b5a9-422c-b92c-27403f2d008b,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-10-28 11:17:31.28229,,,,,t,2025-10-28 11:17:31.28229,,,
5bcfce58-783f-499f-a5e0-e8ba24181a83,9276e4d9-7c52-4710-99d6-80a3e0bccab2,274ebe13-cf0a-48c7-8da0-d9b16f54aa6b,37f5a98e-b5a9-422c-b92c-27403f2d008b,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 11:17:31.28229,,,,,t,2025-10-28 11:17:31.28229,,,
ff95a96a-3e03-4d31-b773-4bdd9839053b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,e11ab8a0-f041-4e25-ac38-77f87797c5f2,4380f2f7-f8dc-46a8-b8fb-b5a05de9abbb,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-10-28 11:27:21.399564,,,,,t,2025-10-28 11:27:21.399564,,,
771fa1c7-84ff-4fba-8d41-e8afe85eb86b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,e11ab8a0-f041-4e25-ac38-77f87797c5f2,4380f2f7-f8dc-46a8-b8fb-b5a05de9abbb,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 11:27:21.399564,,,,,t,2025-10-28 11:27:21.399564,,,
1ac2fc80-9593-4928-ae81-7b95d730bf91,9276e4d9-7c52-4710-99d6-80a3e0bccab2,e11ab8a0-f041-4e25-ac38-77f87797c5f2,4380f2f7-f8dc-46a8-b8fb-b5a05de9abbb,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 11:27:21.399564,,,,,t,2025-10-28 11:27:21.399564,,,
e78c7335-c555-40ac-a94a-a9e9fd5b471d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,e11ab8a0-f041-4e25-ac38-77f87797c5f2,4380f2f7-f8dc-46a8-b8fb-b5a05de9abbb,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 11:27:21.399564,,,,,t,2025-10-28 11:27:21.399564,,,
bbbaf779-0af3-4b59-9ddc-cc6c4ff3e3e5,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2d67758d-0ff9-4098-85f2-333b9211d601,629679fd-482d-4104-beca-183427b35b0e,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 11:32:34.011901,,,,,t,2025-10-28 11:32:34.011901,,,
a29c6723-c251-49b4-8e8e-15de37dab5de,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2d67758d-0ff9-4098-85f2-333b9211d601,629679fd-482d-4104-beca-183427b35b0e,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 11:32:34.011901,,,,,t,2025-10-28 11:32:34.011901,,,
4a61588d-aa76-42d0-b4df-6fb80fde0573,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2d67758d-0ff9-4098-85f2-333b9211d601,629679fd-482d-4104-beca-183427b35b0e,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-10-28 11:32:34.011901,,,,,t,2025-10-28 11:32:34.011901,,,
45315d51-340e-4c83-b612-df3aee185df6,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2d67758d-0ff9-4098-85f2-333b9211d601,629679fd-482d-4104-beca-183427b35b0e,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 11:32:34.011901,,,,,t,2025-10-28 11:32:34.011901,,,
64a14a68-2571-4046-b0e1-ffff3f60915d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2a3fd199-9e5b-412d-9359-c4215559afec,6475f4a0-88fc-4d10-9352-e5d97910c83e,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 11:59:36.778436,,,,,t,2025-10-28 11:59:36.778436,,,
11ad7a02-2220-4fe4-956d-dd26f46cb02c,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2a3fd199-9e5b-412d-9359-c4215559afec,6475f4a0-88fc-4d10-9352-e5d97910c83e,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-10-28 11:59:36.778436,,,,,t,2025-10-28 11:59:36.778436,,,
8d6a0616-74dc-4708-bb09-24db0b0507dc,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2a3fd199-9e5b-412d-9359-c4215559afec,6475f4a0-88fc-4d10-9352-e5d97910c83e,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 11:59:36.778436,,,,,t,2025-10-28 11:59:36.778436,,,
624c7e68-9609-41d8-87c4-4790420c56b9,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2a3fd199-9e5b-412d-9359-c4215559afec,6475f4a0-88fc-4d10-9352-e5d97910c83e,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 11:59:36.778436,,,,,t,2025-10-28 11:59:36.778436,,,
3b2dc670-baea-4356-ac84-84ae5b3073c0,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5bc1df49-767c-427c-b750-9b60c097bbdf,947f2f4c-df83-4eef-a7bc-836431bdd0e2,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 12:08:26.355055,,,,,t,2025-10-28 12:08:26.355055,,,
c6f583ba-294e-4af7-9457-9122afbe8dbf,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5bc1df49-767c-427c-b750-9b60c097bbdf,947f2f4c-df83-4eef-a7bc-836431bdd0e2,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 12:08:26.355055,,,,,t,2025-10-28 12:08:26.355055,,,
aee156c8-c99f-40d7-a5a2-70ab51c4c8ea,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5bc1df49-767c-427c-b750-9b60c097bbdf,947f2f4c-df83-4eef-a7bc-836431bdd0e2,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 12:08:26.355055,,,,,t,2025-10-28 12:08:26.355055,,,
955680e3-ea36-4e67-bf66-bfa779d8c55c,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5bc1df49-767c-427c-b750-9b60c097bbdf,947f2f4c-df83-4eef-a7bc-836431bdd0e2,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,in_progress,f,2025-10-28 12:08:26.355055,7950f4cf-081a-42f7-90fa-8805e46623fd,,,,t,2025-10-28 12:08:26.355055,2025-10-28 12:09:13.144837,,
95fc76b3-51cb-4892-8520-bfd1fd9123f4,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,b5c71b98-b57e-4c53-b9d0-92a254fccd50,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-10-28 13:26:39.357963,,,,,t,2025-10-28 13:26:39.357963,,,
e020b9b5-51f8-4707-ae1d-ec2660dc7de7,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,b5c71b98-b57e-4c53-b9d0-92a254fccd50,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-10-28 13:26:39.357963,,,,,t,2025-10-28 13:26:39.357963,,,
5f5c8bfe-6a80-49f0-b5c6-8d9457ef5426,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,b5c71b98-b57e-4c53-b9d0-92a254fccd50,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 13:26:39.357963,,,,,t,2025-10-28 13:26:39.357963,,,
5bb5a98d-ac23-44aa-a648-bd6c3aeceb9e,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,98d9e63c-f3cf-4dda-9383-d47b0d72385b,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-11-11 05:13:28.908967,,,,,t,2025-11-11 05:13:28.908967,,,
4d5f10f3-8d2f-458d-92af-bbaf1295c893,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,850930b4-3486-4d8e-9635-168b61d230d0,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,in_progress,f,2025-10-28 14:25:19.904596,988f28bf-086c-4772-812d-8a6018cf1d55,,,,t,2025-10-28 14:25:19.904596,2025-10-28 14:31:56.724002,https://goddard.fillout.com/t/mNy14Tpfu1us?_t=SAEiSezzDOQdU8yvoJlwHhqhbSOXiqpv&student_form_assignment_id=4d5f10f3-8d2f-458d-92af-bbaf1295c893,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI4NGQ4YzM3YS0zZGVlLTRjODUtYjZjYi01NWE0YWIxNzkzZDMiLCJkb2N1bWVudElkIjoiMTI3YWVlZmYtODFlZS00YTQ4LWE0ZWMtMTk2NWUyYzRiNjAxIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJtTnkxNFRwZnUxdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY2MTkxNX0.uUJJevSOgDdujxcwLXjNYyL4xh6JNb-bN__CVjrB_Uc
f7954f19-3425-4fef-a55e-8e11ac19758f,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,850930b4-3486-4d8e-9635-168b61d230d0,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,in_progress,f,2025-10-28 14:25:19.904596,1dc0c5ee-492c-4541-8b61-7c4aec5779c3,,,,t,2025-10-28 14:25:19.904596,2025-10-28 14:38:25.759511,https://goddard.fillout.com/t/uxYBSkibvFus?_t=MhhznrHYshCr9ZFvunsOPmlMJHlQLMrR&student_form_assignment_id=f7954f19-3425-4fef-a55e-8e11ac19758f,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiIzNTQxMmE0OC00ZGZjLTRhZDYtYmIyZC0wZjgxYzhmZGJjMGMiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY2MjMwM30.0NcRRym3T7XwrSjSInH73fTTYRCE0AZjGGTCszH2pjg
aa7abe91-7f75-4b45-b61e-f286742e5b1e,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,b5c71b98-b57e-4c53-b9d0-92a254fccd50,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,in_progress,f,2025-10-28 13:26:39.357963,eacd1979-99ac-4896-a46e-da0e70060ab0,,,,t,2025-10-28 13:26:39.357963,2025-10-28 13:37:51.452661,https://goddard.fillout.com/t/uxYBSkibvFus?_t=RGtgsU3KLSGq6GvIVJ9812J72hLgy8CX&student_form_assignment_id=aa7abe91-7f75-4b45-b61e-f286742e5b1e,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI2YWIzOGNmMy0zMWM5LTQyNTctYjFjNi0xNTNhNTIwMDNjYWQiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY1ODY3MH0.lxQyGhE6hdQvWgB6s-7fLRevI1y0bF6_6cNnyLqvrFE
7115e078-e125-4096-bae1-39b94ed2ed46,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,850930b4-3486-4d8e-9635-168b61d230d0,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-10-28 14:25:19.904596,,,,,t,2025-10-28 14:25:19.904596,,,
0d2351fb-fd30-46bd-8d75-669f90c94c12,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,850930b4-3486-4d8e-9635-168b61d230d0,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,in_progress,f,2025-10-28 14:25:19.904596,fb5293a7-6ad1-406e-9b03-ef945a2232e9,,,,t,2025-10-28 14:25:19.904596,2025-10-28 14:29:10.191098,https://goddard.fillout.com/t/gXRdaCT2rKus?_t=PkpTNAuZqexI5BFEGH5xulBLPRYEkVda&student_form_assignment_id=0d2351fb-fd30-46bd-8d75-669f90c94c12,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiJkYTExNDk2OS02NDFkLTRkNzktODZiYy0yYjdiODY4OWM3ZWYiLCJkb2N1bWVudElkIjoiNmI2MjFkNTgtMDcxYi00YWFlLThiNjAtOTlmYjQ3YjcwYzc4IiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJnWFJkYUNUMnJLdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY2MTc0N30.T1Nkc6Mjo4m5RMzbUTsQ_sWKRY8jYAqyL1OxjyvzMag
10604e91-e3e9-44f5-ac11-6820d9862f3e,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,98d9e63c-f3cf-4dda-9383-d47b0d72385b,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-11-11 05:13:28.908967,,,,,t,2025-11-11 05:13:28.908967,,,
f476d83c-7ba8-4fa2-b4ef-d73b6098d9a4,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2b674756-a7dc-44ba-a242-c77653c805d8,67a90ee3-aaee-4c1d-9836-453eb9dc6a92,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-11-11 05:31:11.238186,,,,,t,2025-11-11 05:31:11.238186,,,
bbd7d05d-7c1f-4ab4-b250-71dc55beb32a,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2b674756-a7dc-44ba-a242-c77653c805d8,67a90ee3-aaee-4c1d-9836-453eb9dc6a92,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-11-11 05:31:11.238186,,,,,t,2025-11-11 05:31:11.238186,,,
1a6e7009-0192-481b-909b-6f101fd027e1,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2b674756-a7dc-44ba-a242-c77653c805d8,67a90ee3-aaee-4c1d-9836-453eb9dc6a92,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-11-11 05:31:11.238186,,,,,t,2025-11-11 05:31:11.238186,,,
5506e9fd-98bc-4ef4-8be8-6ebaebed0673,9276e4d9-7c52-4710-99d6-80a3e0bccab2,2b674756-a7dc-44ba-a242-c77653c805d8,67a90ee3-aaee-4c1d-9836-453eb9dc6a92,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-11-11 05:31:11.238186,,,,,t,2025-11-11 05:31:11.238186,,,
3f6ef4ba-402a-4075-97df-b3626be127cd,9276e4d9-7c52-4710-99d6-80a3e0bccab2,896a8bf8-261b-441e-9547-b7647b269ee6,5dff0e9d-97af-4945-bb2b-8d69166a17fa,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-11-11 05:37:06.868462,,,,,t,2025-11-11 05:37:06.868462,,,
bd9f803a-ccbf-40c9-a4f0-af2fd6d4f313,9276e4d9-7c52-4710-99d6-80a3e0bccab2,896a8bf8-261b-441e-9547-b7647b269ee6,5dff0e9d-97af-4945-bb2b-8d69166a17fa,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-11-11 05:37:06.868462,,,,,t,2025-11-11 05:37:06.868462,,,
a147b44d-3247-44cd-8b67-d152085588ad,9276e4d9-7c52-4710-99d6-80a3e0bccab2,896a8bf8-261b-441e-9547-b7647b269ee6,5dff0e9d-97af-4945-bb2b-8d69166a17fa,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-11-11 05:37:06.868462,,,,,t,2025-11-11 05:37:06.868462,,,
1b4e96a6-2283-458e-9793-fbd76979a9b0,9276e4d9-7c52-4710-99d6-80a3e0bccab2,896a8bf8-261b-441e-9547-b7647b269ee6,5dff0e9d-97af-4945-bb2b-8d69166a17fa,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-11-11 05:37:06.868462,,,,,t,2025-11-11 05:37:06.868462,,,
85140086-55a5-472c-ae27-2085d3a49623,9276e4d9-7c52-4710-99d6-80a3e0bccab2,a8bedec4-2988-4021-b9db-6f85f575601a,48a8848e-c984-431d-8827-91078ca8d679,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,rejected,f,2025-11-11 14:35:13.560205,2a0a5b3c-d6d5-4691-acf3-f3d90b782812,6eae1b0e-d835-4590-871d-e781f798f3e1,test,2025-11-11 14:41:31.013216,t,2025-11-11 14:35:13.560205,2025-11-11 14:41:31.013216,https://goddard.fillout.com/t/mNy14Tpfu1us?_t=Ya3DQ5rJ2RHdZPrCQqq6kHbICCYh9DZj&student_form_assignment_id=85140086-55a5-472c-ae27-2085d3a49623,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI2Mzk2NThlNS1iMDI1LTQ0ODQtYTRkNS1iYWQ3NjU4YTNiYTYiLCJkb2N1bWVudElkIjoiMTI3YWVlZmYtODFlZS00YTQ4LWE0ZWMtMTk2NWUyYzRiNjAxIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJtTnkxNFRwZnUxdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg3MjAyOH0.FN5knNHK9IwtyxWxUmASnL5qTLPOVBJ0I95B_L0Rxfk
5cb3f01c-4a55-4e0c-b0da-338f91be54e7,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,98d9e63c-f3cf-4dda-9383-d47b0d72385b,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,approved,f,2025-11-11 05:13:28.908967,00ee5e7f-86f9-4585-81ab-0703636e775d,6eae1b0e-d835-4590-871d-e781f798f3e1,All good,2025-11-11 13:02:10.653981,t,2025-11-11 05:13:28.908967,2025-11-11 13:02:10.653981,https://goddard.fillout.com/t/mNy14Tpfu1us?_t=ua5YXYtdZsSmvbgsKda3o9Iv7tVQMEbl&student_form_assignment_id=5cb3f01c-4a55-4e0c-b0da-338f91be54e7,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiJjNmVmNDc5Mi1lYjM2LTQyYmEtYjM0YS1iOTk2M2ZjYTk1N2IiLCJkb2N1bWVudElkIjoiMTI3YWVlZmYtODFlZS00YTQ4LWE0ZWMtMTk2NWUyYzRiNjAxIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJtTnkxNFRwZnUxdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg1NDUwOX0.Ym1buugFMmQCD2fK1i81M6G-D7s0nrKn72UEAOG7F7A
ba54a870-4aea-4db1-976c-540b225d9f09,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,98d9e63c-f3cf-4dda-9383-d47b0d72385b,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,approved,f,2025-11-11 05:13:28.908967,75a2ad2a-5c7a-48f0-ba2f-bd39e0a3ec5c,6eae1b0e-d835-4590-871d-e781f798f3e1,"",2025-11-11 14:32:39.571349,t,2025-11-11 05:13:28.908967,2025-11-11 14:32:39.571349,https://goddard.fillout.com/t/uxYBSkibvFus?_t=glSo89VdamBt5hkLifVhpzSmeUkRPrLQ&student_form_assignment_id=ba54a870-4aea-4db1-976c-540b225d9f09,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI4OTEzZjVmNi1hNjI4LTQxMzItYTYwNi0wZGI0Y2YwMjg2ZGEiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg0OTU1MH0.JD0XSQmvYlZP1mAaStfvX3j_T9qz7rWndgWOLDs4EzE
44e96b57-59dd-4216-a2a7-733bb03d735f,9276e4d9-7c52-4710-99d6-80a3e0bccab2,a8bedec4-2988-4021-b9db-6f85f575601a,48a8848e-c984-431d-8827-91078ca8d679,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-11-11 14:35:13.560205,,,,,t,2025-11-11 14:35:13.560205,,,
0ce90977-722d-46ff-bafc-b7501c04c466,9276e4d9-7c52-4710-99d6-80a3e0bccab2,c94d23d7-45ae-4521-8544-0546bb90087a,1be77d9d-7c92-4a0f-af21-b1639ddc983b,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2025-11-11 10:03:10.736494,,,,,t,2025-11-11 10:03:10.736494,,,
17c204e2-84f7-4bd4-b85d-c2fbb346dc2d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,c94d23d7-45ae-4521-8544-0546bb90087a,1be77d9d-7c92-4a0f-af21-b1639ddc983b,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2025-11-11 10:03:10.736494,,,,,t,2025-11-11 10:03:10.736494,,,
a7e01bfd-f910-451b-91e4-024497a540e2,9276e4d9-7c52-4710-99d6-80a3e0bccab2,c94d23d7-45ae-4521-8544-0546bb90087a,1be77d9d-7c92-4a0f-af21-b1639ddc983b,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2025-11-11 10:03:10.736494,,,,,t,2025-11-11 10:03:10.736494,,,
890da141-8136-45ed-a72e-b602eaa78af6,9276e4d9-7c52-4710-99d6-80a3e0bccab2,c94d23d7-45ae-4521-8544-0546bb90087a,1be77d9d-7c92-4a0f-af21-b1639ddc983b,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-11-11 10:03:10.736494,,,,,t,2025-11-11 10:03:10.736494,,,
99ec342f-be7f-48f8-b627-bcedff4d2bda,9276e4d9-7c52-4710-99d6-80a3e0bccab2,a8bedec4-2988-4021-b9db-6f85f575601a,48a8848e-c984-431d-8827-91078ca8d679,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2025-11-11 14:35:13.560205,,,,,t,2025-11-11 14:35:13.560205,,,
8be778a4-e76b-42b0-b36e-58d0fcb83e7f,9276e4d9-7c52-4710-99d6-80a3e0bccab2,eb76631f-c9c9-4f6d-83e3-b0c35a52f8eb,31870db7-1ccb-42ee-8227-f16a3a2a894b,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2026-02-08 09:08:58.810576,,,,,t,2026-02-08 09:08:58.810576,,,
a46f93ce-b457-4bd1-a2fd-b88ec80604b1,9276e4d9-7c52-4710-99d6-80a3e0bccab2,eb76631f-c9c9-4f6d-83e3-b0c35a52f8eb,31870db7-1ccb-42ee-8227-f16a3a2a894b,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2026-02-08 09:08:58.810576,,,,,t,2026-02-08 09:08:58.810576,,,
db56e1ce-c31c-4d88-8185-cd9cb439d35b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,a8bedec4-2988-4021-b9db-6f85f575601a,48a8848e-c984-431d-8827-91078ca8d679,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,in_progress,f,2025-11-11 14:35:13.560205,b9d5208b-6203-470d-9228-e25eba756814,,,,t,2025-11-11 14:35:13.560205,2025-11-11 14:48:49.52647,https://goddard.fillout.com/t/uxYBSkibvFus?_t=W5FpYIBVZ89o6AvfXcnaraB7FQdGUBFp&student_form_assignment_id=db56e1ce-c31c-4d88-8185-cd9cb439d35b,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiIzMTc5NDY0My1kYTE2LTQwODQtOTRmZC02YWIxYzU2MTRjMzgiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg3MjUyN30.1xBbseOMVUutQPok2JbK21N-kuV8qiteY9I2n0ZtFUo
69ac46ec-18df-4303-893a-060b8bed0fc8,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f4d0b93e-4c29-4b95-83b9-a445add82224,bcfc9031-2902-4fac-9c1c-48aad08624ee,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,school_default,incomplete,f,2026-02-07 16:39:17.808288,,,,,t,2026-02-07 16:39:17.808288,,,
4a67d6ca-2d7a-4d40-8675-4720004c9f56,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f4d0b93e-4c29-4b95-83b9-a445add82224,bcfc9031-2902-4fac-9c1c-48aad08624ee,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2026-02-07 16:39:17.808288,,,,,t,2026-02-07 16:39:17.808288,,,
e13ba7d9-05d5-4203-a0af-86f7e92e74da,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f4d0b93e-4c29-4b95-83b9-a445add82224,bcfc9031-2902-4fac-9c1c-48aad08624ee,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2026-02-07 16:39:17.808288,,,,,t,2026-02-07 16:39:17.808288,,,
524e7c17-8ead-45a4-b50e-66b28840613b,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f4d0b93e-4c29-4b95-83b9-a445add82224,bcfc9031-2902-4fac-9c1c-48aad08624ee,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,school_default,incomplete,f,2026-02-07 16:39:17.808288,,,,,t,2026-02-07 16:39:17.808288,,,
4057f8c3-e06e-4247-8f52-fab6fe415d4c,9276e4d9-7c52-4710-99d6-80a3e0bccab2,eb76631f-c9c9-4f6d-83e3-b0c35a52f8eb,31870db7-1ccb-42ee-8227-f16a3a2a894b,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,school_default,incomplete,f,2026-02-08 09:08:58.810576,,,,,t,2026-02-08 09:08:58.810576,,,
bc6bd1ac-70f7-4291-99d8-1bbaa4f49d8d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,eb76631f-c9c9-4f6d-83e3-b0c35a52f8eb,31870db7-1ccb-42ee-8227-f16a3a2a894b,35f76c68-eb06-47c3-8888-a4230bdafd99,school_default,incomplete,f,2026-02-08 09:08:58.810576,,,,,t,2026-02-08 09:08:58.810576,,,
\.

-- Data for table: form_submissions
-- Rows:     15
\COPY form_submissions FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,school_id,enrollment_id,student_form_assignment_id,form_template_id,fillout_submission_id,form_data,metadata,submitted_at,processed_at,is_active,created_at,updated_at,edit_link,pdf_link
7950f4cf-081a-42f7-90fa-8805e46623fd,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5bc1df49-767c-427c-b750-9b60c097bbdf,955680e3-ea36-4e67-bf66-bfa779d8c55c,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,c0c988eb-fc89-4072-94aa-73768baf24d3,"{""Date"": ""2025-10-15"", ""State"": ""Tamil Nadu"", ""form_id"": ""tdKTQWnb3Wus"", ""Bank Routing"": 25668888, ""Driver's License"": ""HFHF757"", ""Parent Signature"": ""GDFDFD"", ""form_submission_id"": ""c0c988eb-fc89-4072-94aa-73768baf24d3"", ""Authorization ACH Bank Account"": 657676}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 12:09:12.999442359"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-10-28 12:09:12.999442,2025-10-28 12:09:12.999442,t,2025-10-28 12:09:12.999442,2025-10-28 12:09:12.999442,,
3a958f3f-0efd-4b56-98a5-f45f02b18bd2,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,aa7abe91-7f75-4b45-b61e-f286742e5b1e,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,0847ac93-d90d-482d-af7c-5593b6c4acc0,"{""Date"": ""2025-10-09"", ""State"": ""Tamil Nadu"", ""form_id"": ""tdKTQWnb3Wus"", ""Bank Routing"": 7897, ""Driver's License"": ""678fghy687t"", ""Parent Signature"": ""ygy"", ""form_submission_id"": ""0847ac93-d90d-482d-af7c-5593b6c4acc0"", ""Authorization ACH Bank Account"": 6767867}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 13:28:20.710590143"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-10-28 13:28:20.71059,2025-10-28 13:28:20.71059,t,2025-10-28 13:28:20.71059,2025-10-28 13:28:20.71059,,
b0e6d26a-2206-4f25-9a04-6923be4db43d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,aa7abe91-7f75-4b45-b61e-f286742e5b1e,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,85ac4b2c-09d9-402f-a68d-4fb8faecd9bb,"{""Date"": ""2025-10-01"", ""State"": ""Kerala"", ""form_id"": ""tdKTQWnb3Wus"", ""Bank Routing"": 75665, ""Driver's License"": ""GJH756657"", ""Parent Signature"": ""MANI"", ""form_submission_id"": ""85ac4b2c-09d9-402f-a68d-4fb8faecd9bb"", ""Authorization ACH Bank Account"": 67556}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 13:35:22.028914924"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-10-28 13:35:22.028915,2025-10-28 13:35:22.028915,t,2025-10-28 13:35:22.028915,2025-10-28 13:35:22.028915,,
eacd1979-99ac-4896-a46e-da0e70060ab0,9276e4d9-7c52-4710-99d6-80a3e0bccab2,1cd2966f-5497-4470-9570-48923c6fbc83,aa7abe91-7f75-4b45-b61e-f286742e5b1e,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,6ab38cf3-31c9-4257-b1c6-153a52003cad,"{""Date"": ""2025-10-15"", ""State"": ""Tamil Nadu"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 76876, ""Driver's License"": ""gf867"", ""Parent Signature"": ""gfgf"", ""form_submission_id"": ""6ab38cf3-31c9-4257-b1c6-153a52003cad"", ""Authorization ACH Bank Account"": 768768}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 13:37:50.737867531"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-10-28 13:37:50.737868,2025-10-28 13:37:50.737868,t,2025-10-28 13:37:50.737868,2025-10-28 13:37:51.369319,https://goddard.fillout.com/t/uxYBSkibvFus?_t=RGtgsU3KLSGq6GvIVJ9812J72hLgy8CX&student_form_assignment_id=aa7abe91-7f75-4b45-b61e-f286742e5b1e,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI2YWIzOGNmMy0zMWM5LTQyNTctYjFjNi0xNTNhNTIwMDNjYWQiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY1ODY3MH0.lxQyGhE6hdQvWgB6s-7fLRevI1y0bF6_6cNnyLqvrFE
fb5293a7-6ad1-406e-9b03-ef945a2232e9,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,0d2351fb-fd30-46bd-8d75-669f90c94c12,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,da114969-641d-4d79-86bc-2b7b8689c7ef,"{""dob"": ""2025-10-08"", ""date"": ""2025-10-27T18:30:00.000Z"", ""email"": ""abc@gmail.com"", ""form_id"": ""gXRdaCT2rKus"", ""fifth_sign"": ""hujk"", ""first_sign"": ""dfjd"", ""ninth_sign"": ""hjk"", ""sixth_sign"": ""ikjk"", ""tenth_form"": ""ujkmk"", ""third_sign"": ""jjkj"", ""fourth_sign"": ""ijiokkm"", ""parent_sign"": ""logi"", ""second_sign"": ""gfuhj"", ""eightth_sign"": ""kjkj"", ""home_address"": ""56/dhdjkljuer"", ""seventh_sign"": ""bjjk"", ""twelfth_sign"": ""lkk"", ""children_name"": ""Reena"", ""eleventh_sign"": ""hjkkl"", ""fourteen_sign"": ""hijkkjj"", ""thirteen_sign"": ""hjkkkl"", ""fifteenth_sign"": ""hjbkklk"", ""sixteenth_sign"": ""mbgu"", ""eighteenth_sign"": ""nkk"", ""nignteenth_sign"": ""hkjkl"", ""seventeenth_sign"": ""ujklj"", ""form_submission_id"": ""da114969-641d-4d79-86bc-2b7b8689c7ef"", ""preferred_start_date"": ""nmdsskl""}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 14:29:08.388931772"", ""webhook_payload_keys"": [""children_name"", ""date"", ""dob"", ""eighteenth_sign"", ""eightth_sign"", ""eleventh_sign"", ""email"", ""fifteenth_sign"", ""fifth_sign"", ""first_sign"", ""form_id"", ""form_submission_id"", ""fourteen_sign"", ""fourth_sign"", ""home_address"", ""nignteenth_sign"", ""ninth_sign"", ""parent_sign"", ""preferred_start_date"", ""second_sign"", ""seventeenth_sign"", ""seventh_sign"", ""sixteenth_sign"", ""sixth_sign"", ""student_form_assignment_id"", ""tenth_form"", ""third_sign"", ""thirteen_sign"", ""twelfth_sign""]}",2025-10-28 14:29:08.388932,2025-10-28 14:29:08.388932,t,2025-10-28 14:29:08.388932,2025-10-28 14:29:10.107342,https://goddard.fillout.com/t/gXRdaCT2rKus?_t=PkpTNAuZqexI5BFEGH5xulBLPRYEkVda&student_form_assignment_id=0d2351fb-fd30-46bd-8d75-669f90c94c12,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiJkYTExNDk2OS02NDFkLTRkNzktODZiYy0yYjdiODY4OWM3ZWYiLCJkb2N1bWVudElkIjoiNmI2MjFkNTgtMDcxYi00YWFlLThiNjAtOTlmYjQ3YjcwYzc4IiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJnWFJkYUNUMnJLdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY2MTc0N30.T1Nkc6Mjo4m5RMzbUTsQ_sWKRY8jYAqyL1OxjyvzMag
988f28bf-086c-4772-812d-8a6018cf1d55,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,4d5f10f3-8d2f-458d-92af-bbaf1295c893,35f76c68-eb06-47c3-8888-a4230bdafd99,84d8c37a-3dee-4c85-b6cb-55a4ab1793d3,"{""form_id"": ""mNy14Tpfu1us"", ""sec_note"": true, ""fifth_note"": true, ""sixth_note"": true, ""third_note"": true, ""eighth_note"": true, ""nineth_note"": true, ""parent_sign"": ""logi"", ""tentth_note"": true, ""seventh_note"": true, ""twelfth_note"": true, ""welcome_note"": true, ""eleventh_note"": true, ""fifteenth_note"": true, ""fourtenth_note"": true, ""sixteenth_note"": true, ""eighteenth_note"": true, ""nineteenth_note"": true, ""thirtrrnth_note"": true, ""seventeenth_note"": true, ""form_submission_id"": ""84d8c37a-3dee-4c85-b6cb-55a4ab1793d3"", ""parent_signed_date"": ""2025-10-28""}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 14:31:55.989169583"", ""webhook_payload_keys"": [""eighteenth_note"", ""eighth_note"", ""eleventh_note"", ""fifteenth_note"", ""fifth_note"", ""form_id"", ""form_submission_id"", ""fourtenth_note"", ""nineteenth_note"", ""nineth_note"", ""parent_sign"", ""parent_signed_date"", ""sec_note"", ""seventeenth_note"", ""seventh_note"", ""sixteenth_note"", ""sixth_note"", ""student_form_assignment_id"", ""tentth_note"", ""third_note"", ""thirtrrnth_note"", ""twelfth_note"", ""welcome_note""]}",2025-10-28 14:31:55.98917,2025-10-28 14:31:55.98917,t,2025-10-28 14:31:55.98917,2025-10-28 14:31:56.640973,https://goddard.fillout.com/t/mNy14Tpfu1us?_t=SAEiSezzDOQdU8yvoJlwHhqhbSOXiqpv&student_form_assignment_id=4d5f10f3-8d2f-458d-92af-bbaf1295c893,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI4NGQ4YzM3YS0zZGVlLTRjODUtYjZjYi01NWE0YWIxNzkzZDMiLCJkb2N1bWVudElkIjoiMTI3YWVlZmYtODFlZS00YTQ4LWE0ZWMtMTk2NWUyYzRiNjAxIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJtTnkxNFRwZnUxdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY2MTkxNX0.uUJJevSOgDdujxcwLXjNYyL4xh6JNb-bN__CVjrB_Uc
1dc0c5ee-492c-4541-8b61-7c4aec5779c3,9276e4d9-7c52-4710-99d6-80a3e0bccab2,41143921-eb7b-4f8d-b529-8f1f950cd094,f7954f19-3425-4fef-a55e-8e11ac19758f,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,35412a48-4dfc-4ad6-bb2d-0f81c8fdbc0c,"{""Date"": ""2025-10-28"", ""State"": ""Kerala"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 4455, ""Driver's License"": ""5263653"", ""Parent Signature"": ""logi"", ""form_submission_id"": ""35412a48-4dfc-4ad6-bb2d-0f81c8fdbc0c"", ""Authorization ACH Bank Account"": 55655}","{""source"": ""webhook"", ""received_at"": ""2025-10-28 14:38:24.812914995"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-10-28 14:38:24.812915,2025-10-28 14:38:24.812915,t,2025-10-28 14:38:24.812915,2025-10-28 14:38:25.676134,https://goddard.fillout.com/t/uxYBSkibvFus?_t=MhhznrHYshCr9ZFvunsOPmlMJHlQLMrR&student_form_assignment_id=f7954f19-3425-4fef-a55e-8e11ac19758f,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiIzNTQxMmE0OC00ZGZjLTRhZDYtYmIyZC0wZjgxYzhmZGJjMGMiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2MTY2MjMwM30.0NcRRym3T7XwrSjSInH73fTTYRCE0AZjGGTCszH2pjg
78b37bae-3e98-4408-b878-5f5e95fd8040,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,ba54a870-4aea-4db1-976c-540b225d9f09,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,01dd5403-89b4-41e2-915d-1258b88d76cc,"{""Date"": ""2025-11-11"", ""State"": ""Alaska"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 8237275, ""Driver's License"": ""sdtytwhr"", ""Parent Signature"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-uxYBSkibvFus/226c205c-7eac-4fdb-944f-18df2bfd76d9-L8UVarPm7tph67jo0qr5wDfOMYv7FZZh6Ahmlg6udd9Vykqu3cAez4xK7bvhzUamxxnvVl3JnS5IWxcqXMEzoSMneUUq27GQkIJ/signature_uxYBSkibvFus_Tue-Nov-11-2025-114401-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_uxYBSkibvFus_Tue Nov 11 2025 11:44:01 GMT+0530 (India Standard Time).png""}], ""form_submission_id"": ""01dd5403-89b4-41e2-915d-1258b88d76cc"", ""Authorization ACH Bank Account"": 92834627}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 06:14:09.564203931"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-11-11 06:14:09.564204,2025-11-11 06:14:09.564204,t,2025-11-11 06:14:09.564204,2025-11-11 06:14:09.564204,,
95956806-544d-48ac-ab87-2e048d1808ba,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,ba54a870-4aea-4db1-976c-540b225d9f09,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,125a20e6-f1f8-445a-ba59-008a3fbb5a65,"{""Date"": ""2025-11-11"", ""State"": ""Alaska"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 787887, ""Driver's License"": ""7887GHFFY"", ""Parent Signature"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-uxYBSkibvFus/16a2c7ca-c894-4aeb-b264-da9aed42e5a2-h1rslPcauerhkHHkMSvVM7aCdvLBz7YgOngcMAcx0hQu39LxHUaksa19GjtZVqCK13aknfwSvlRz3L4Vv43iAsQlvJAd1fNec79/signature_uxYBSkibvFus_Tue-Nov-11-2025-134154-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_uxYBSkibvFus_Tue Nov 11 2025 13:41:54 GMT+0530 (India Standard Time).png""}], ""form_submission_id"": ""125a20e6-f1f8-445a-ba59-008a3fbb5a65"", ""Authorization ACH Bank Account"": 878787}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 08:12:05.279485931"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-11-11 08:12:05.279486,2025-11-11 08:12:05.279486,t,2025-11-11 08:12:05.279486,2025-11-11 08:12:05.279486,,
7d1b216c-9385-4387-8653-0406d4dcf742,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,ba54a870-4aea-4db1-976c-540b225d9f09,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,b09133de-347c-42da-8ff2-b83e464b634b,"{""Date"": ""2025-11-11"", ""State"": ""California"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 87789978789, ""Driver's License"": ""0009VH98009"", ""Parent Signature"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-uxYBSkibvFus/d870cb51-429e-4262-85a4-632df0d8f2ca-X2jbgJqtEL3hbUHcmKrnBhGebwq6d8afiMdKPvE2AY3Q1gETqny2fvEidiTkx5mhhaV8R3B32zEuRnGAwAgWmFPtkowthuKLh11/signature_uxYBSkibvFus_Tue-Nov-11-2025-134532-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_uxYBSkibvFus_Tue Nov 11 2025 13:45:32 GMT+0530 (India Standard Time).png""}], ""form_submission_id"": ""b09133de-347c-42da-8ff2-b83e464b634b"", ""Authorization ACH Bank Account"": 9009090}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 08:15:37.908254726"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-11-11 08:15:37.908255,2025-11-11 08:15:37.908255,t,2025-11-11 08:15:37.908255,2025-11-11 08:15:37.908255,,
8201629c-ad47-4d26-9120-73e4c754f618,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,ba54a870-4aea-4db1-976c-540b225d9f09,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,f125857e-148f-4122-8f34-89539ae0a95a,"{""Date"": ""2025-11-11"", ""State"": ""Utah"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 34534656, ""Driver's License"": ""TFTYMRNTYKJR"", ""Parent Signature"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-uxYBSkibvFus/251147ef-4c9f-4505-a348-2b61306cc95d-j1ZHWtH28LPMTebrs6tz4wBLvsOzGZDQ7yhg4W7ystWdz98cASd1Qpms7wW6oKJ0SC9Z62KB8vPKiB9t4vKNkT8PHi5urx4JGfr/signature_uxYBSkibvFus_Tue-Nov-11-2025-134842-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_uxYBSkibvFus_Tue Nov 11 2025 13:48:42 GMT+0530 (India Standard Time).png""}], ""form_submission_id"": ""f125857e-148f-4122-8f34-89539ae0a95a"", ""Authorization ACH Bank Account"": 4564757}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 08:18:49.835447445"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-11-11 08:18:49.835447,2025-11-11 08:18:49.835447,t,2025-11-11 08:18:49.835447,2025-11-11 08:18:49.835447,,
75a2ad2a-5c7a-48f0-ba2f-bd39e0a3ec5c,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,ba54a870-4aea-4db1-976c-540b225d9f09,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,8913f5f6-a628-4132-a606-0db4cf0286da,"{""Date"": ""2025-11-11"", ""State"": ""Arizona"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 8989, ""Driver's License"": ""989898"", ""Parent Signature"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-uxYBSkibvFus/d837a254-1e57-437a-bf38-3494ebda9942-G0G3rDpeQdWS1KrfS1wgz5xI557ReVuLHXcSjKO2gypIH1HtXNgQtS8LHXKWTDCnkjKnsMqsFfey7a0JkVnNdRWIDG4rrwdn0Z3/signature_uxYBSkibvFus_Tue-Nov-11-2025-135543-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_uxYBSkibvFus_Tue Nov 11 2025 13:55:43 GMT+0530 (India Standard Time).png""}], ""form_submission_id"": ""8913f5f6-a628-4132-a606-0db4cf0286da"", ""Authorization ACH Bank Account"": 9898}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 08:25:50.730844800"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-11-11 08:25:50.730845,2025-11-11 08:25:50.730845,t,2025-11-11 08:25:50.730845,2025-11-11 08:25:51.40414,https://goddard.fillout.com/t/uxYBSkibvFus?_t=glSo89VdamBt5hkLifVhpzSmeUkRPrLQ&student_form_assignment_id=ba54a870-4aea-4db1-976c-540b225d9f09,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI4OTEzZjVmNi1hNjI4LTQxMzItYTYwNi0wZGI0Y2YwMjg2ZGEiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg0OTU1MH0.JD0XSQmvYlZP1mAaStfvX3j_T9qz7rWndgWOLDs4EzE
00ee5e7f-86f9-4585-81ab-0703636e775d,9276e4d9-7c52-4710-99d6-80a3e0bccab2,f0015897-3515-44bf-9f3c-19c113749066,5cb3f01c-4a55-4e0c-b0da-338f91be54e7,35f76c68-eb06-47c3-8888-a4230bdafd99,c6ef4792-eb36-42ba-b34a-b9963fca957b,"{""form_id"": ""mNy14Tpfu1us"", ""sec_note"": true, ""fifth_note"": true, ""sixth_note"": true, ""third_note"": true, ""eighth_note"": true, ""nineth_note"": true, ""parent_sign"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-mNy14Tpfu1us/6262434e-77f8-47d1-b947-c9050adb328c-kvQPNWT7NMkTcaPKTutcWnSYdcFXk8AIsWaWlg8kyutgvEtdeJFnb7hqgbL6QjKBUomE11NUanHnfi86fKj10KU0wUr2vN6j9rn/signature_mNy14Tpfu1us_Tue-Nov-11-2025-151822-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_mNy14Tpfu1us_Tue Nov 11 2025 15:18:22 GMT+0530 (India Standard Time).png""}], ""tentth_note"": true, ""seventh_note"": true, ""twelfth_note"": true, ""welcome_note"": true, ""eleventh_note"": true, ""fifteenth_note"": true, ""fourtenth_note"": true, ""sixteenth_note"": true, ""eighteenth_note"": true, ""nineteenth_note"": true, ""thirtrrnth_note"": true, ""seventeenth_note"": true, ""form_submission_id"": ""c6ef4792-eb36-42ba-b34a-b9963fca957b"", ""parent_signed_date"": ""2025-11-11""}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 09:48:29.578758922"", ""webhook_payload_keys"": [""eighteenth_note"", ""eighth_note"", ""eleventh_note"", ""fifteenth_note"", ""fifth_note"", ""form_id"", ""form_submission_id"", ""fourtenth_note"", ""nineteenth_note"", ""nineth_note"", ""parent_sign"", ""parent_signed_date"", ""sec_note"", ""seventeenth_note"", ""seventh_note"", ""sixteenth_note"", ""sixth_note"", ""student_form_assignment_id"", ""tentth_note"", ""third_note"", ""thirtrrnth_note"", ""twelfth_note"", ""welcome_note""]}",2025-11-11 09:48:29.578759,2025-11-11 09:48:29.578759,t,2025-11-11 09:48:29.578759,2025-11-11 09:48:30.359267,https://goddard.fillout.com/t/mNy14Tpfu1us?_t=ua5YXYtdZsSmvbgsKda3o9Iv7tVQMEbl&student_form_assignment_id=5cb3f01c-4a55-4e0c-b0da-338f91be54e7,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiJjNmVmNDc5Mi1lYjM2LTQyYmEtYjM0YS1iOTk2M2ZjYTk1N2IiLCJkb2N1bWVudElkIjoiMTI3YWVlZmYtODFlZS00YTQ4LWE0ZWMtMTk2NWUyYzRiNjAxIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJtTnkxNFRwZnUxdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg1NDUwOX0.Ym1buugFMmQCD2fK1i81M6G-D7s0nrKn72UEAOG7F7A
2a0a5b3c-d6d5-4691-acf3-f3d90b782812,9276e4d9-7c52-4710-99d6-80a3e0bccab2,a8bedec4-2988-4021-b9db-6f85f575601a,85140086-55a5-472c-ae27-2085d3a49623,35f76c68-eb06-47c3-8888-a4230bdafd99,639658e5-b025-4484-a4d5-bad7658a3ba6,"{""form_id"": ""mNy14Tpfu1us"", ""sec_note"": true, ""fifth_note"": true, ""sixth_note"": true, ""third_note"": true, ""eighth_note"": true, ""nineth_note"": true, ""parent_sign"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-mNy14Tpfu1us/30aa9be4-689a-4550-beba-7cca861439e0-9cvO2LMgQCukych0Zfh8wVSxE7Ur8Qv8TC3GeYHL6MKM8B2KmsXDhGsuWP5N8a3C75fQJhmLysZHRxgiqm56le4r6M1dweiw6wT/signature_mNy14Tpfu1us_Tue-Nov-11-2025-201016-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_mNy14Tpfu1us_Tue Nov 11 2025 20:10:16 GMT+0530 (India Standard Time).png""}], ""tentth_note"": true, ""seventh_note"": true, ""twelfth_note"": true, ""welcome_note"": true, ""eleventh_note"": true, ""fifteenth_note"": true, ""fourtenth_note"": true, ""sixteenth_note"": true, ""eighteenth_note"": true, ""nineteenth_note"": true, ""thirtrrnth_note"": true, ""seventeenth_note"": true, ""form_submission_id"": ""639658e5-b025-4484-a4d5-bad7658a3ba6"", ""parent_signed_date"": ""2025-11-11""}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 14:40:28.934890163"", ""webhook_payload_keys"": [""eighteenth_note"", ""eighth_note"", ""eleventh_note"", ""fifteenth_note"", ""fifth_note"", ""form_id"", ""form_submission_id"", ""fourtenth_note"", ""nineteenth_note"", ""nineth_note"", ""parent_sign"", ""parent_signed_date"", ""sec_note"", ""seventeenth_note"", ""seventh_note"", ""sixteenth_note"", ""sixth_note"", ""student_form_assignment_id"", ""tentth_note"", ""third_note"", ""thirtrrnth_note"", ""twelfth_note"", ""welcome_note""]}",2025-11-11 14:40:28.93489,2025-11-11 14:40:28.93489,t,2025-11-11 14:40:28.93489,2025-11-11 14:40:29.809351,https://goddard.fillout.com/t/mNy14Tpfu1us?_t=Ya3DQ5rJ2RHdZPrCQqq6kHbICCYh9DZj&student_form_assignment_id=85140086-55a5-472c-ae27-2085d3a49623,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiI2Mzk2NThlNS1iMDI1LTQ0ODQtYTRkNS1iYWQ3NjU4YTNiYTYiLCJkb2N1bWVudElkIjoiMTI3YWVlZmYtODFlZS00YTQ4LWE0ZWMtMTk2NWUyYzRiNjAxIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJtTnkxNFRwZnUxdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg3MjAyOH0.FN5knNHK9IwtyxWxUmASnL5qTLPOVBJ0I95B_L0Rxfk
b9d5208b-6203-470d-9228-e25eba756814,9276e4d9-7c52-4710-99d6-80a3e0bccab2,a8bedec4-2988-4021-b9db-6f85f575601a,db56e1ce-c31c-4d88-8185-cd9cb439d35b,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,31794643-da16-4084-94fd-6ab1c5614c38,"{""Date"": ""2025-11-11"", ""State"": ""Delaware"", ""form_id"": ""uxYBSkibvFus"", ""Bank Routing"": 23235, ""Driver's License"": ""fdsfr"", ""Parent Signature"": [{""url"": ""https://prod-fillout-oregon-s3.s3.us-west-2.amazonaws.com/orgid-446157/flowpublicid-uxYBSkibvFus/f0ef8df4-d012-4094-b542-3b20ccf0ad1d-ZyLcMq28wADtQhA79SdbXpJR6pz6Rj74DisDtTq3g4j1ZW0CvkZZByEsd0LuOCS2bq7L1e8R98i5SKmGXPRBdKxKz8QyJOPX4fZ/signature_uxYBSkibvFus_Tue-Nov-11-2025-201834-GMT0530-(India-Standard-Time).png"", ""filename"": ""signature_uxYBSkibvFus_Tue Nov 11 2025 20:18:34 GMT+0530 (India Standard Time).png""}], ""form_submission_id"": ""31794643-da16-4084-94fd-6ab1c5614c38"", ""Authorization ACH Bank Account"": 23424}","{""source"": ""webhook"", ""received_at"": ""2025-11-11 14:48:48.852412295"", ""webhook_payload_keys"": [""Authorization ACH Bank Account"", ""Bank Routing"", ""Date"", ""Driver's License"", ""Parent Signature"", ""State"", ""form_id"", ""form_submission_id"", ""student_form_assignment_id""]}",2025-11-11 14:48:48.852412,2025-11-11 14:48:48.852412,t,2025-11-11 14:48:48.852412,2025-11-11 14:48:49.441633,https://goddard.fillout.com/t/uxYBSkibvFus?_t=W5FpYIBVZ89o6AvfXcnaraB7FQdGUBFp&student_form_assignment_id=db56e1ce-c31c-4d88-8185-cd9cb439d35b,https://api.fillout.com/v1/files/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJtaXNzaW9uSWQiOiIzMTc5NDY0My1kYTE2LTQwODQtOTRmZC02YWIxYzU2MTRjMzgiLCJkb2N1bWVudElkIjoiMDQzZmQyMWQtMTRjMC00ODc0LTk1ZTEtMDFjYzI1ZDI2OGUyIiwiZmxvd1B1YmxpY0lkZW50aWZpZXIiOiJ1eFlCU2tpYnZGdXMiLCJtb2RlIjoibGl2ZSIsImlhdCI6MTc2Mjg3MjUyN30.1xBbseOMVUutQPok2JbK21N-kuV8qiteY9I2n0ZtFUo
\.

-- Data for table: documents
-- Rows:      0

-- Data for table: class_form_overrides
-- Rows:      6
\COPY class_form_overrides FROM STDIN WITH (FORMAT CSV, HEADER true, DELIMITER ',', QUOTE '"', ESCAPE '"');
id,school_id,classroom_id,form_template_id,action,is_required,created_at,updated_at,is_active
28c9dbff-6b30-4c50-9a19-20af6d916ed9,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5d265168-72d0-4304-9c84-973d969a6557,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,,,2025-11-11 05:23:57.741594,,t
c7ac5f0b-2cca-42f6-abf6-3970e6bb6da1,9276e4d9-7c52-4710-99d6-80a3e0bccab2,5d265168-72d0-4304-9c84-973d969a6557,35f76c68-eb06-47c3-8888-a4230bdafd99,,,2025-11-11 05:23:57.74564,,t
6f418bb2-ddf1-4d8f-8975-eebeed7a47c6,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,7eb674c0-0bcf-48d4-8f0f-745fd29feaee,,,2026-02-07 16:34:06.550707,,t
09f18276-8005-497f-b7fe-ca44a6edb28c,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,35f76c68-eb06-47c3-8888-a4230bdafd99,,,2026-02-07 16:34:06.549198,,t
34c0a2c1-49ff-4324-a81f-069cf58fc132,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,0b9ccb32-19cc-4634-82d2-9d6e81ea527e,,,2026-02-07 16:34:06.559655,,t
3fb9f42c-dc8e-4710-b10d-3c6e927e3ed7,9276e4d9-7c52-4710-99d6-80a3e0bccab2,26464a03-3824-4b9c-8b38-fd1f7e5b6deb,b3eb3b74-26e9-42bc-b8eb-ec998c5d9d5a,,,2026-02-07 16:34:06.803496,,t
\.

-- Backup completed at Tue Feb 10 14:12:40 IST 2026
