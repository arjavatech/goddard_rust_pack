# The Goddard School Enrollment Management System
## High-Level Architecture Design Document

### Executive Summary
This document presents a comprehensive architectural design for The Goddard School Enrollment Management System using React.js hosted on Cloudflare Pages for the frontend, Rust with AWS Lambda for the backend, and Supabase for database, authentication, and real-time services. Email communications are handled by Resend API. The architecture is designed for an initial launch of 10 schools with 50 concurrent users each, prioritizing scalability, security, multi-tenancy, and compliance with childcare regulations (COPPA/FERPA).

---

## 1. System Architecture Overview

```mermaid
graph TB
    subgraph "Client Layer"
        WEB[React.js Web App]
        MOBILE[Mobile Responsive]
    end
    
    subgraph "Cloudflare Infrastructure"
        CF_PAGES[Cloudflare Pages]
        CF_CDN[Cloudflare CDN]
        CF_WAF[Cloudflare WAF]
    end
    
    subgraph "API Gateway Layer"
        APIGW[AWS API Gateway]
        WAF[AWS WAF]
    end
    
    subgraph "Form Management Layer"
        FILLOUT[Fillout.com Platform]
        FILLOUT_API[Fillout REST API]
        CUSTOM_DOMAINS[School Custom Domains]
        FORM_EMBED[Embedded Forms]
    end
    
    subgraph "Serverless Compute Layer"
        subgraph "Lambda Functions (Rust)"
            AUTH_FN[Auth Service]
            ENROLL_FN[Enrollment Service]
            FORM_WEBHOOK_FN[Form Webhook Handler]
            DOC_FN[Document Service]
            NOTIF_FN[Notification Service]
            ADMIN_FN[Admin Service]
            APPROVAL_FN[Admin Approval Service]
            FORM_LOCK_FN[Form Locking Service]
            PARENT_EMAIL_FN[Parent Email Management Service]
            MULTI_NOTIF_FN[Multi-Email Notification Service]
        end
    end
    
    subgraph "Data Layer"
        subgraph "Supabase Platform"
            SUPA_AUTH[Supabase Auth]
            SUPA_DB[(PostgreSQL + RLS)]
            SUPA_STORAGE[Supabase Storage]
            SUPA_RT[Realtime Subscriptions]
        end
    end
    
    subgraph "Integration Services"
        RESEND[Resend Email API]
        S3_DOCS[S3 Documents]
        LAMBDA_PDF[PDF Generator]
        CW[CloudWatch]
    end
    
    WEB --> CF_PAGES
    MOBILE --> CF_PAGES
    CF_PAGES --> CF_CDN
    CF_CDN --> CF_WAF
    
    WEB --> FORM_EMBED
    FORM_EMBED --> FILLOUT
    FILLOUT --> CUSTOM_DOMAINS
    FILLOUT -.->|Webhooks| APIGW
    
    WEB --> APIGW
    MOBILE --> APIGW
    WAF --> APIGW
    APIGW --> AUTH_FN
    APIGW --> ENROLL_FN
    APIGW --> FORM_WEBHOOK_FN
    APIGW --> DOC_FN
    APIGW --> NOTIF_FN
    APIGW --> ADMIN_FN
    APIGW --> APPROVAL_FN
    APIGW --> FORM_LOCK_FN
    APIGW --> PARENT_EMAIL_FN
    APIGW --> MULTI_NOTIF_FN
    
    AUTH_FN --> SUPA_AUTH
    ENROLL_FN --> SUPA_DB
    FORM_WEBHOOK_FN --> SUPA_DB
    DOC_FN --> SUPA_STORAGE
    DOC_FN --> S3_DOCS
    NOTIF_FN --> RESEND
    ADMIN_FN --> SUPA_DB
    ADMIN_FN --> FILLOUT_API
    APPROVAL_FN --> SUPA_DB
    APPROVAL_FN --> SUPA_RT
    FORM_LOCK_FN --> SUPA_DB
    PARENT_EMAIL_FN --> SUPA_DB
    MULTI_NOTIF_FN --> RESEND
    MULTI_NOTIF_FN --> PARENT_EMAIL_FN
    
    ENROLL_FN --> LAMBDA_PDF
    FORM_WEBHOOK_FN --> SUPA_RT
    ALL_FN --> CW
```

---

## 2. Multi-Tenant Architecture

### 2.1 Tenant Isolation Strategy (Manual Onboarding)

```mermaid
graph LR
    subgraph "Manual School Setup"
        ADMIN[Super Admin]
        SCHOOL_FORM[School Registration Form]
        DB_SETUP[Database Schema Setup]
    end
    
    subgraph "Tenant Context"
        JWT[JWT with school_id]
        SUPABASE_AUTH[Supabase Auth Policies]
    end
    
    subgraph "Data Isolation"
        RLS[Row Level Security]
        SCHOOL_COL[school_id column]
    end
    
    ADMIN --> SCHOOL_FORM
    SCHOOL_FORM --> DB_SETUP
    DB_SETUP --> JWT
    JWT --> SUPABASE_AUTH
    SUPABASE_AUTH --> RLS
    RLS --> SCHOOL_COL
```

**Manual Onboarding Process:**
1. Super Admin creates new school record
2. School subdomain and basic settings configured
3. Initial admin user created for the school
4. RLS policies automatically apply data isolation
5. School admin can begin customizing forms and settings

### 2.2 Database Schema with Multi-Tenancy & Form Management

```mermaid
erDiagram
    SCHOOLS ||--o{ USERS : has
    SCHOOLS ||--o{ CLASSROOMS : contains
    SCHOOLS ||--o{ FORM_TEMPLATES : manages
    SCHOOLS {
        uuid id PK
        string name
        string subdomain UK
        jsonb settings
        timestamp created_at
    }
    
    USERS ||--o{ CHILDREN : has
    USERS ||--o{ ENROLLMENTS : creates
    USERS ||--o{ PARENT_ADDITIONAL_EMAILS : has
    USERS {
        uuid id PK
        uuid school_id FK
        string email UK
        string role
        jsonb metadata
    }
    
    PARENT_ADDITIONAL_EMAILS {
        uuid id PK
        uuid school_id FK
        uuid parent_id FK
        string email_address
        string email_type
        boolean is_verified
        boolean is_active
        uuid added_by FK
        text notes
        timestamp created_at
    }
    
    CHILDREN ||--|| ENROLLMENTS : enrolled_via
    CHILDREN {
        uuid id PK
        uuid parent_id FK
        uuid school_id FK
        string first_name
        string last_name
        date birth_date
        jsonb medical_info
    }
    
    ENROLLMENTS ||--o{ STUDENT_FORM_ASSIGNMENTS : has
    ENROLLMENTS {
        uuid id PK
        uuid child_id FK
        uuid school_id FK
        uuid classroom_id FK
        string status
        jsonb progress
        timestamp submitted_at
    }
    
    FORM_TEMPLATES ||--|| SCHOOLS : belongs_to
    FORM_TEMPLATES ||--o{ STUDENT_FORM_ASSIGNMENTS : assigned_via
    FORM_TEMPLATES ||--o{ CLASS_FORM_OVERRIDES : customized_by
    FORM_TEMPLATES ||--o{ FORM_SUBMISSIONS : generates
    FORM_TEMPLATES {
        uuid id PK
        uuid school_id FK
        string form_name
        string form_type
        string fillout_form_id
        string fillout_form_url
        string status
        boolean is_required
        integer display_order
        timestamp created_at
    }
    
    CLASSROOMS ||--o{ ENROLLMENTS : contains
    CLASSROOMS ||--o{ CLASS_FORM_OVERRIDES : has
    CLASSROOMS {
        uuid id PK
        uuid school_id FK
        string name
        string age_group
        integer capacity
        integer enrolled_count
    }
    
    CLASS_FORM_OVERRIDES {
        uuid id PK
        uuid school_id FK
        uuid classroom_id FK
        uuid form_template_id FK
        string action
        boolean is_required
        timestamp created_at
    }
    
    STUDENT_FORM_ASSIGNMENTS ||--|| FORM_SUBMISSIONS : completed_via
    STUDENT_FORM_ASSIGNMENTS {
        uuid id PK
        uuid school_id FK
        uuid enrollment_id FK
        uuid child_id FK
        uuid form_template_id FK
        string assignment_source
        boolean is_required
        timestamp assigned_at
    }
    
    FORM_SUBMISSIONS {
        uuid id PK
        uuid school_id FK
        uuid enrollment_id FK
        uuid student_form_assignment_id FK
        uuid form_template_id FK
        string fillout_submission_id UK
        jsonb form_data
        jsonb metadata
        timestamp submitted_at
        timestamp processed_at
    }
    
    DOCUMENTS ||--|| ENROLLMENTS : attached_to
    DOCUMENTS {
        uuid id PK
        uuid enrollment_id FK
        uuid school_id FK
        string document_type
        string storage_path
        string file_name
        timestamp uploaded_at
    }
```

---

## 3. Technical Stack Details

### 3.1 Frontend Architecture (React.js)

```mermaid
graph TD
    subgraph "React Application Structure"
        APP[App.tsx]
        
        subgraph "Core Modules"
            AUTH_MOD[Auth Module]
            ENROLL_MOD[Enrollment Module]
            ADMIN_MOD[Admin Module]
            PARENT_EMAIL_MOD[Parent Email Module]
        end
        
        subgraph "Shared Components"
            FORMS[Form Components]
            UI[UI Library]
            LAYOUTS[Layout Components]
        end
        
        subgraph "State Management"
            CONTEXT[React Context]
            QUERY[React Query]
            LOCAL[Local Storage]
        end
        
        subgraph "Services"
            API[API Client]
            SUPA_CLIENT[Supabase Client]
            VALIDATORS[Form Validators]
        end
    end
    
    APP --> AUTH_MOD
    APP --> ENROLL_MOD
    APP --> ADMIN_MOD
    APP --> PARENT_EMAIL_MOD
    
    AUTH_MOD --> CONTEXT
    ENROLL_MOD --> FORMS
    ADMIN_MOD --> UI
    
    CONTEXT --> API
    QUERY --> SUPA_CLIENT
```

#### Frontend Technology Decisions:
- **Framework**: React 18 with TypeScript
- **Hosting**: Cloudflare Pages with automatic deployments
- **Routing**: React Router v6
- **State Management**: React Context + React Query
- **UI Framework**: Tailwind CSS + Shadcn/ui
- **Form Management**: Fillout.com embedded forms + React integration
- **Form Builder**: Fillout.com Business plan with custom domains
- **Real-time**: Supabase Realtime subscriptions
- **Build Tool**: Vite
- **CDN**: Cloudflare's global edge network

### 3.2 Backend Architecture (Rust + AWS Lambda)

```mermaid
graph LR
    subgraph "Lambda Function Architecture"
        subgraph "Shared Libraries"
            CORE[goddard-core]
            AUTH_LIB[goddard-auth]
            DB[goddard-db]
        end
        
        subgraph "Lambda Handlers"
            AUTH_H[auth-handler]
            ENROLL_H[enrollment-handler]
            FORMS_H[forms-handler]
            DOCS_H[documents-handler]
            PARENT_EMAIL_H[parent-email-handler]
            APPROVAL_H[admin-approval-handler]
            FORM_LOCK_H[form-locking-handler]
            MULTI_NOTIF_H[multi-notification-handler]
        end
        
        subgraph "Utilities"
            VALID[Validators]
            ERRORS[Error Handling]
            LOGGING[Structured Logging]
        end
    end
    
    AUTH_H --> CORE
    AUTH_H --> AUTH_LIB
    ENROLL_H --> CORE
    ENROLL_H --> DB
    FORMS_H --> VALID
    DOCS_H --> ERRORS
    PARENT_EMAIL_H --> DB
    PARENT_EMAIL_H --> VALID
    APPROVAL_H --> CORE
    APPROVAL_H --> DB
    APPROVAL_H --> AUTH_LIB
    FORM_LOCK_H --> DB
    FORM_LOCK_H --> VALID
    MULTI_NOTIF_H --> PARENT_EMAIL_H
    MULTI_NOTIF_H --> CORE
```

#### Backend Technology Decisions:
- **Runtime**: AWS Lambda with Rust Runtime
- **Framework**: Axum for HTTP handling
- **Database Client**: tokio-postgres with connection pooling
- **Serialization**: Serde with JSON
- **Authentication**: JWT validation with jsonwebtoken
- **Webhook Processing**: Fillout webhook validation and processing
- **Form Integration**: Fillout REST API client
- **Approval Workflow**: Database functions with audit trail
- **Form Locking**: Real-time validation with immutable constraints
- **Multi-Email Notifications**: Queue-based processing with delivery tracking
- **Error Handling**: thiserror + anyhow
- **Logging**: tracing with CloudWatch integration
- **Architecture**: Hexagonal/Ports & Adapters

### 3.3 Database & Authentication (Supabase)

```mermaid
graph TD
    subgraph "Supabase Architecture"
        subgraph "Authentication"
            EMAIL_AUTH[Email/Password]
            MAGIC_LINK[Magic Links]
            JWT_GEN[JWT Generation]
        end
        
        subgraph "Database Features"
            RLS_POLICIES[RLS Policies]
            TRIGGERS[Database Triggers]
            FUNCTIONS[Stored Functions]
        end
        
        subgraph "Real-time"
            CHANNELS[Broadcast Channels]
            PRESENCE[Presence Tracking]
            DB_CHANGES[Database Changes]
            APPROVAL_EVENTS[Approval Status Changes]
            FORM_LOCK_EVENTS[Form Lock Events]
        end
        
        subgraph "Storage"
            BUCKET_PUBLIC[Public Bucket]
            BUCKET_PRIVATE[Private Bucket]
            POLICIES_STORAGE[Storage Policies]
        end
    end
```

---

## 4. Security Architecture

### 4.1 Security Layers

```mermaid
graph TB
    subgraph "Network Security"
        WAF[AWS WAF]
        CLOUDFRONT[CloudFront DDoS Protection]
        VPC[VPC Security Groups]
    end
    
    subgraph "Application Security"
        JWT_VAL[JWT Validation]
        RATE_LIMIT[Rate Limiting]
        INPUT_VAL[Input Validation]
        CORS[CORS Policy]
    end
    
    subgraph "Data Security"
        RLS_SEC[Row Level Security]
        ENCRYPTION[Encryption at Rest]
        TLS[TLS in Transit]
        FIELD_ENCRYPT[Field-level Encryption]
    end
    
    subgraph "Compliance"
        COPPA[COPPA Compliance]
        FERPA[FERPA Compliance]
        AUDIT[Audit Logging]
        RETENTION[Data Retention]
    end
```

### 4.2 Authentication & Authorization Flow

```mermaid
sequenceDiagram
    participant User
    participant React
    participant APIGateway
    participant Lambda
    participant Supabase
    participant Database
    
    User->>React: Login Request
    React->>Supabase: Authenticate
    Supabase->>Supabase: Validate Credentials
    Supabase-->>React: JWT Token
    React->>React: Store Token
    
    User->>React: Access Protected Resource
    React->>APIGateway: Request + JWT
    APIGateway->>Lambda: Forward Request
    Lambda->>Lambda: Validate JWT
    Lambda->>Lambda: Extract tenant_id
    Lambda->>Database: Query with RLS
    Database->>Database: Apply RLS Policy
    Database-->>Lambda: Filtered Data
    Lambda-->>APIGateway: Response
    APIGateway-->>React: Protected Data
    React-->>User: Display Data
```

---

## 5. Data Flow Diagrams

### 5.1 Enrollment Process Flow

```mermaid
graph TD
    START[Parent Receives Invitation] --> EMAIL[Email with Magic Link]
    EMAIL --> VERIFY[Email Verification]
    VERIFY --> SCHOOL[School Selection/Confirmation]
    SCHOOL --> ACCOUNT[Account Creation]
    ACCOUNT --> CHILD[Add Child Information]
    
    CHILD --> FORMS[Form Selection]
    FORMS --> FILL[Fill Forms]
    FILL --> SAVE[Auto-save Progress]
    SAVE --> VALIDATE[Validate Input]
    VALIDATE --> UPLOAD[Upload Documents]
    
    UPLOAD --> REVIEW[Review Application]
    REVIEW --> SUBMIT[Submit Enrollment]
    SUBMIT --> NOTIFY_ADMIN[Notify Admin]
    
    NOTIFY_ADMIN --> ADMIN_REVIEW[Admin Reviews]
    ADMIN_REVIEW --> APPROVE{Approved?}
    APPROVE -->|Yes| ASSIGN[Assign Classroom]
    APPROVE -->|No| REQUEST_INFO[Request More Info]
    
    ASSIGN --> CONFIRM[Send Confirmation]
    REQUEST_INFO --> FILL
```

### 5.2 Form Management Flow

```mermaid
stateDiagram-v2
    [*] --> Draft: Create Form
    Draft --> InProgress: Start Filling
    InProgress --> InProgress: Auto-save
    InProgress --> Validated: All Fields Valid
    Validated --> Submitted: Submit
    Submitted --> UnderReview: Admin Notified
    UnderReview --> Approved: Admin Approves
    UnderReview --> Rejected: Admin Rejects
    Rejected --> InProgress: Resubmit
    Approved --> Completed: Process Complete
    Completed --> [*]
```

---

## 6. Infrastructure Architecture

### 6.1 Hybrid Infrastructure (Cloudflare + AWS)

```mermaid
graph TB
    subgraph "Cloudflare Infrastructure"
        CF_PAGES[Cloudflare Pages]
        CF_GLOBAL[Global Edge Network]
        CF_WAF[Cloudflare WAF]
        CF_ANALYTICS[Cloudflare Analytics]
    end
    
    subgraph "AWS Account"
        subgraph "Primary Region (us-west-2)"
            subgraph "API Layer"
                APIGW_MAIN[API Gateway]
                WAF_REGIONAL[AWS WAF]
            end
            
            subgraph "Compute"
                LAMBDA_WARM[Warm Lambda Pool]
                LAMBDA_COLD[Cold Lambda Instances]
            end
            
            subgraph "Storage"
                S3_DOCS_MAIN[S3 Documents]
                S3_BACKUP[S3 Backups]
            end
            
            subgraph "Monitoring"
                CW_LOGS[CloudWatch Logs]
                CW_METRICS[CloudWatch Metrics]
                XRAY[X-Ray Tracing]
            end
        end
        
        subgraph "External Services"
            RESEND_API[Resend Email Service]
            SUPABASE[Supabase Platform]
        end
    end
    
    CF_PAGES --> CF_GLOBAL
    CF_GLOBAL --> CF_WAF
    CF_PAGES -.->|API Calls| APIGW_MAIN
```

### 6.2 Deployment Pipeline (Cloudflare Pages + AWS)

```mermaid
graph LR
    subgraph "Development"
        DEV_BRANCH[Feature Branch]
        LOCAL[Local Testing]
    end
    
    subgraph "CI/CD Pipeline"
        GH_ACTIONS[GitHub Actions]
        BUILD_FE[Build Frontend]
        BUILD_BE[Build Backend]
        SECURITY[Security Scan]
    end
    
    subgraph "Frontend Deployment"
        CF_PREVIEW[Cloudflare Preview]
        CF_PROD[Cloudflare Production]
    end
    
    subgraph "Backend Deployment"
        AWS_DEV[AWS Dev]
        AWS_STAGING[AWS Staging]
        AWS_PROD[AWS Production]
    end
    
    DEV_BRANCH --> GH_ACTIONS
    GH_ACTIONS --> BUILD_FE
    GH_ACTIONS --> BUILD_BE
    BUILD_FE --> SECURITY
    BUILD_BE --> SECURITY
    
    SECURITY --> CF_PREVIEW
    SECURITY --> AWS_DEV
    CF_PREVIEW --> CF_PROD
    AWS_DEV --> AWS_STAGING
    AWS_STAGING --> AWS_PROD
```

---

## 7. Performance Optimization Strategies

### 7.1 Frontend Optimization
- **Code Splitting**: Lazy load routes and heavy components
- **Bundle Optimization**: Tree shaking, minification
- **Caching Strategy**: Service workers for offline support
- **Image Optimization**: WebP format, lazy loading
- **CDN Distribution**: Static assets via CloudFront

### 7.2 Backend Optimization
- **Lambda Optimization**:
  - Provisioned concurrency for critical functions
  - ARM64 architecture (30% cost reduction)
  - Minimal binary size with release builds
  - Connection pooling with RDS Proxy pattern
  
- **Database Optimization**:
  - Indexed columns for RLS policies
  - Materialized views for reports
  - Connection pooling
  - Query optimization

### 7.3 Caching Architecture

```mermaid
graph TD
    subgraph "Cache Layers"
        BROWSER[Browser Cache]
        CDN_CACHE[CDN Cache]
        API_CACHE[API Gateway Cache]
        REDIS[Redis Cache Layer]
        DB_CACHE[Database Query Cache]
    end
    
    BROWSER --> CDN_CACHE
    CDN_CACHE --> API_CACHE
    API_CACHE --> REDIS
    REDIS --> DB_CACHE
```

---

## 8. Technical Decisions & Trade-offs

### 8.1 Key Technology Choices

| Component | Technology | Rationale | Trade-offs |
|-----------|------------|-----------|------------|
| **Frontend** | React.js | - Large ecosystem<br>- Component reusability<br>- Strong community | - Bundle size<br>- SEO challenges |
| **Backend** | Rust + Lambda | - 6x faster than Python<br>- Memory safety<br>- Cost efficient | - Longer build times<br>- Smaller talent pool |
| **Database** | Supabase/PostgreSQL | - Built-in RLS<br>- Real-time capabilities<br>- Managed service | - Vendor lock-in<br>- Limited customization |
| **Auth** | Supabase Auth | - Seamless DB integration<br>- Built-in RLS support<br>- Multiple auth providers<br>- JWT + RLS policies | - Single vendor dependency<br>- Less enterprise features |
| **Infrastructure** | AWS Serverless | - Auto-scaling<br>- Pay-per-use<br>- No server management | - Cold starts<br>- Vendor lock-in |

### 8.2 Architectural Patterns

```mermaid
graph TD
    subgraph "Design Patterns"
        HEX[Hexagonal Architecture]
        CQRS[CQRS Pattern]
        EVENT[Event Sourcing]
        SAGA[Saga Pattern]
    end
    
    subgraph "Implementation"
        PORTS[Ports & Adapters]
        COMMANDS[Command Handlers]
        EVENTS_STORE[Event Store]
        ORCHESTRATOR[Saga Orchestrator]
    end
    
    HEX --> PORTS
    CQRS --> COMMANDS
    EVENT --> EVENTS_STORE
    SAGA --> ORCHESTRATOR
```

---

## 9. Risk Analysis & Mitigation

### 9.1 Technical Risks

| Risk | Impact | Probability | Mitigation Strategy |
|------|--------|-------------|-------------------|
| **Lambda Cold Starts** | High | Medium | - Provisioned concurrency<br>- Warm-up functions<br>- Optimize bundle size |
| **Supabase Downtime** | Critical | Low | - Multi-region setup<br>- Backup database<br>- Disaster recovery plan |
| **Data Breach** | Critical | Low | - Encryption everywhere<br>- RLS policies<br>- Security audits<br>- PII tokenization |
| **Scaling Issues** | High | Medium | - Load testing<br>- Auto-scaling policies<br>- Performance monitoring |
| **Compliance Violation** | Critical | Low | - Regular audits<br>- Automated compliance checks<br>- Data retention policies |

### 9.2 Mitigation Architecture

```mermaid
graph LR
    subgraph "Prevention"
        SEC_REVIEW[Security Reviews]
        LOAD_TEST[Load Testing]
        COMPLIANCE[Compliance Checks]
    end
    
    subgraph "Detection"
        MONITORING[Real-time Monitoring]
        ALERTS[Alert System]
        AUDIT_LOG[Audit Logging]
    end
    
    subgraph "Response"
        INCIDENT[Incident Response]
        ROLLBACK[Rollback Plan]
        DR_PLAN[DR Activation]
    end
    
    SEC_REVIEW --> MONITORING
    LOAD_TEST --> ALERTS
    COMPLIANCE --> AUDIT_LOG
    MONITORING --> INCIDENT
    ALERTS --> ROLLBACK
    AUDIT_LOG --> DR_PLAN
```

---

## 10. Implementation Roadmap

### 10.1 Phase 1: Foundation (Weeks 1-4)
```mermaid
gantt
    title Phase 1: Foundation
    dateFormat  YYYY-MM-DD
    section Infrastructure
    AWS Setup           :2024-01-01, 3d
    Supabase Setup      :2024-01-02, 2d
    CI/CD Pipeline      :2024-01-04, 3d
    section Backend
    Rust Project Setup  :2024-01-07, 2d
    Core Libraries      :2024-01-09, 5d
    Auth Service        :2024-01-14, 4d
    section Frontend
    React Setup         :2024-01-07, 2d
    Component Library   :2024-01-09, 4d
    Auth Integration    :2024-01-13, 3d
```

### 10.2 Phase 2: Core Features (Weeks 5-8)
```mermaid
gantt
    title Phase 2: Core Features
    dateFormat  YYYY-MM-DD
    section Backend
    Enrollment API      :2024-01-21, 5d
    Forms API          :2024-01-26, 5d
    Document API       :2024-01-31, 4d
    section Frontend
    Enrollment Flow    :2024-01-21, 7d
    Form Components    :2024-01-28, 6d
    Document Upload    :2024-02-03, 3d
    section Multi-Tenancy
    Manual School Setup :2024-01-21, 3d
    RLS Policies      :2024-01-23, 4d
    Tenant Testing    :2024-01-27, 3d
```

### 10.3 Phase 3: Advanced Features (Weeks 9-12)
```mermaid
gantt
    title Phase 3: Advanced Features
    dateFormat  YYYY-MM-DD
    section Features
    Admin Dashboard    :2024-02-12, 7d
    Notifications      :2024-02-19, 4d
    Real-time Updates  :2024-02-23, 4d
    section Integration
    Email Service      :2024-02-12, 3d
    PDF Generation     :2024-02-15, 3d
    Analytics          :2024-02-18, 4d
    section Testing
    Unit Tests         :2024-02-22, 3d
    Integration Tests  :2024-02-25, 3d
    Load Testing       :2024-02-28, 2d
```

---

## 11. Monitoring & Observability

### 11.1 Monitoring Stack

```mermaid
graph TD
    subgraph "Application Metrics"
        APP[Application Logs]
        CUSTOM[Custom Metrics]
        TRACES[Distributed Traces]
    end
    
    subgraph "Infrastructure Metrics"
        LAMBDA_METRICS[Lambda Metrics]
        API_METRICS[API Gateway Metrics]
        DB_METRICS[Database Metrics]
    end
    
    subgraph "Aggregation"
        CW_INSIGHTS[CloudWatch Insights]
        XRAY_TRACE[X-Ray Service Map]
        DASHBOARDS[Custom Dashboards]
    end
    
    subgraph "Alerting"
        SNS[SNS Topics]
        PAGERDUTY[PagerDuty]
        SLACK[Slack Notifications]
    end
    
    APP --> CW_INSIGHTS
    CUSTOM --> DASHBOARDS
    TRACES --> XRAY_TRACE
    
    LAMBDA_METRICS --> DASHBOARDS
    API_METRICS --> DASHBOARDS
    DB_METRICS --> DASHBOARDS
    
    DASHBOARDS --> SNS
    SNS --> PAGERDUTY
    SNS --> SLACK
```

### 11.2 Key Performance Indicators (KPIs) - Optimized for 500 Concurrent Users

| Metric | Target | Alert Threshold | Scaling Notes |
|--------|--------|----------------|---------------|
| **API Response Time (p99)** | < 300ms | > 500ms | Adequate for 500 users |
| **Lambda Cold Start** | < 1s | > 2s | Provisioned concurrency for 10% of capacity |
| **Database Query Time** | < 100ms | > 200ms | Supabase can handle this load easily |
| **Error Rate** | < 0.1% | > 1% | Standard SLA |
| **Form Completion Rate** | > 90% | < 80% | Business success metric |
| **System Uptime** | 99.9% | < 99.5% | 8.76 hours downtime/year max |
| **Concurrent Users** | 500 | > 600 | Auto-scaling threshold |

---

## 12. Cost Optimization

### 12.1 Cost Breakdown Estimation (10 Schools, 500 Concurrent Users)

```mermaid
pie title Monthly Cost Distribution ($750-1100/month)
    "Supabase Pro Plan" : 40
    "Lambda Compute" : 20
    "API Gateway" : 12
    "Fillout Business" : 15
    "S3 Storage" : 7
    "Cloudflare Pages" : 2
    "Resend Email" : 2
    "Monitoring" : 2
```

**Estimated Monthly Costs:**
- **Supabase**: $320 (Pro plan for production + staging)
- **AWS Lambda**: $180 (ARM64, optimized)
- **API Gateway**: $120 (500 concurrent users)
- **Fillout Business Plan**: $150 (5,000+ submissions/month)
- **S3 Storage**: $65 (documents + backups)
- **Cloudflare Pages**: $20 (Pro plan for custom domains)
- **Resend**: $20 (up to 100k emails/month)
- **AWS Monitoring**: $15 (CloudWatch, X-Ray)
- **Total**: ~$750-1100/month

**Additional Cost vs Custom Forms:**
- **+$150/month** for Fillout Business plan
- **Development Savings**: 200+ hours of form development avoided
- **Maintenance Savings**: Ongoing form updates and bug fixes eliminated

### 12.2 Cost Optimization Strategies

1. **Lambda Optimization**:
   - Use ARM64 architecture (30% cheaper)
   - Right-size memory allocation
   - Implement request batching
   - Use Lambda@Edge for simple operations

2. **Database Optimization**:
   - Connection pooling to reduce connections
   - Query optimization and indexing
   - Archive old data to S3

3. **Storage Optimization**:
   - S3 Intelligent-Tiering
   - CloudFront caching
   - Compress documents before storage

---

## 13. Disaster Recovery & Business Continuity

### 13.1 DR Strategy

```mermaid
graph TD
    subgraph "Primary Region"
        PRIMARY_APP[Application]
        PRIMARY_DB[Database]
        PRIMARY_STORAGE[Storage]
    end
    
    subgraph "Backup & Replication"
        CONTINUOUS_BACKUP[Continuous Backup]
        CROSS_REGION[Cross-Region Replication]
        SNAPSHOTS[Daily Snapshots]
    end
    
    subgraph "DR Region"
        DR_APP[Standby Application]
        DR_DB[Replica Database]
        DR_STORAGE[Replicated Storage]
    end
    
    PRIMARY_DB --> CONTINUOUS_BACKUP
    PRIMARY_STORAGE --> CROSS_REGION
    PRIMARY_DB --> SNAPSHOTS
    
    CONTINUOUS_BACKUP --> DR_DB
    CROSS_REGION --> DR_STORAGE
    SNAPSHOTS --> DR_DB
```

### 13.2 Recovery Objectives

- **RTO (Recovery Time Objective)**: 1 hour
- **RPO (Recovery Point Objective)**: 15 minutes
- **Backup Retention**: 30 days
- **Archive Retention**: 7 years (compliance requirement)

---

## 14. Form Management System

### 14.1 Form Hierarchy & Assignment

The system implements a three-tier form assignment hierarchy:

```mermaid
graph TD
    subgraph "School Level"
        SCHOOL_FORMS[School Default Forms]
        ACTIVE_FORMS[Active Forms]
        DRAFT_FORMS[Draft Forms]
        ARCHIVE_FORMS[Archived Forms]
    end
    
    subgraph "Class Level"
        CLASS_INCLUDE[Class Include Overrides]
        CLASS_EXCLUDE[Class Exclude Overrides]
    end
    
    subgraph "Student Level"
        INDIVIDUAL[Individual Assignments]
        MATERIALIZED[Materialized Assignments]
    end
    
    SCHOOL_FORMS --> MATERIALIZED
    ACTIVE_FORMS --> CLASS_INCLUDE
    CLASS_INCLUDE --> MATERIALIZED
    CLASS_EXCLUDE -.->|Removes| MATERIALIZED
    INDIVIDUAL --> MATERIALIZED
```

### 14.2 Form States & Workflow

```mermaid
stateDiagram-v2
    [*] --> Draft: Admin creates form
    Draft --> Active: Admin activates
    Draft --> School_Default: Admin sets as default
    Active --> Archive: Admin archives
    School_Default --> Archive: Admin archives
    Active --> School_Default: Admin promotes
    School_Default --> Active: Admin demotes
    Archive --> [*]: Form retired
    
    note right of School_Default
        Automatically assigned
        to new enrollments
    end note
    
    note right of Active
        Available for manual
        assignment only
    end note
    
    note right of Archive
        No new assignments
        but existing remain
    end note
```

### 14.3 Enrollment Form Assignment Process

```mermaid
sequenceDiagram
    participant Admin
    participant System
    participant Database
    participant Parent
    
    Admin->>System: Create new enrollment
    System->>Database: Get school default forms
    System->>Database: Get class exclusions
    System->>Database: Get class inclusions
    System->>System: Calculate final form list
    System->>Database: Create student_form_assignments
    Database-->>System: Assignment IDs created
    System-->>Admin: Enrollment created with forms
    Admin->>Parent: Send enrollment invitation
    Parent->>System: Access enrollment portal
    System->>Database: Get assigned forms
    Database-->>System: Return form list with URLs
    System-->>Parent: Display forms to complete
```

### Confirmed Requirements
✅ **Authentication**: Supabase Auth (replacing Auth0)  
✅ **Tenant Onboarding**: Manual process by Super Admin  
✅ **Launch Scale**: 10 schools, 50 concurrent users per school (500 total)  
✅ **Data Migration**: Clean start, no legacy data  
✅ **Form Management**: Three-tier hierarchy with materialized assignments  
✅ **Form States**: Active, School Default, Draft, Archive  
✅ **Fillout Integration**: URL/ID storage with webhook processing  

### Remaining Questions for Clarification

#### Technical Details
1. **Document size limits**: Maximum size for uploaded documents?
2. **Real-time requirements**: Which features need instant vs. near real-time updates?
3. **Mobile app timeline**: Native mobile app planned for Phase 2?
4. **Payment integration**: Timeline and requirements?

#### Infrastructure
5. **Data residency**: Any requirements for data location?
6. **Third-party integrations**: Any existing systems to integrate with?
7. **Reporting requirements**: Specific reports needed for launch?

---

## 15. Parent Additional Email Management Feature

### 15.1 Feature Overview

The parent additional email management feature allows school administrators to assign and manage multiple email addresses for parents, enhancing communication reach and providing backup contact methods during the enrollment process.

### 15.2 Key Capabilities

- **Multi-Email Support**: Parents can have multiple email addresses beyond their primary login email
- **Email Types**: Categorize emails as additional, work, backup, emergency contact, etc.
- **Admin Management**: School administrators can add, verify, activate/deactivate additional emails
- **Notification Integration**: All active and verified emails receive enrollment notifications
- **Audit Trail**: Track who added each email and when
- **Multi-Tenant Isolation**: Full RLS support for school data isolation

### 15.3 Database Schema Addition

The `parent_additional_emails` table extends the existing user management system:

```sql
PARENT_ADDITIONAL_EMAILS {
    uuid id PK
    uuid school_id FK
    uuid parent_id FK
    string email_address
    string email_type
    boolean is_verified
    boolean is_active
    uuid added_by FK
    text notes
    timestamp created_at
}
```

### 15.4 API Endpoints

- `GET /api/parents/{parentId}/emails` - Get all emails for a parent
- `POST /api/parents/emails` - Add additional email
- `PUT /api/parents/emails/{emailId}` - Update email status/notes
- `DELETE /api/parents/emails/{emailId}` - Remove additional email
- `GET /api/parents/{parentId}/notification-emails` - Get notification-ready emails
- `GET /api/enrollments/{enrollmentId}/notification-emails` - Get enrollment notification emails

### 15.5 Security & Compliance

- **Row Level Security**: All additional email data isolated by school_id
- **Admin-Only Management**: Only school admins can manage additional emails
- **Email Validation**: Server-side email format validation
- **Audit Logging**: Track all email management actions
- **COPPA/FERPA Compliance**: Additional emails follow same data protection standards

### 15.6 Integration Points

- **Notification System**: Enhanced to use all active parent emails
- **Enrollment Process**: Notifications sent to all verified email addresses
- **Admin Dashboard**: Email management interface for parent records
- **Email Service (Resend)**: Multi-recipient support for parent communications

### 15.7 Business Benefits

- **Improved Communication Reach**: Reduce missed notifications due to email issues
- **Parental Flexibility**: Support work emails, backup addresses, emergency contacts
- **Administrative Control**: Centralized email management per school
- **Enhanced Reliability**: Multiple contact paths for critical enrollment communications

## 16. Admin Approval & Form Locking Architecture

### 16.1 Feature Overview

The admin approval and form locking system provides a comprehensive workflow for school administrators to review, approve, reject, or request revisions for enrollment applications. Once approved, all forms are automatically locked to prevent unauthorized changes, ensuring data integrity and compliance.

### 16.2 Core Components

#### 16.2.1 Admin Approval Service Architecture
```mermaid
graph TD
    subgraph "Admin Approval Workflow"
        SUBMIT_QUEUE[Submission Queue]
        APPROVAL_ENGINE[Approval Engine]
        AUDIT_LOGGER[Audit Logger]
        NOTIFICATION_DISPATCHER[Notification Dispatcher]
    end
    
    subgraph "Form Locking Service"
        LOCK_VALIDATOR[Lock Validator]
        IMMUTABLE_STORE[Immutable Storage]
        LOCK_MONITOR[Lock Monitor]
    end
    
    subgraph "Multi-Email Notification Service"
        EMAIL_COLLECTOR[Email Collector]
        QUEUE_PROCESSOR[Queue Processor]
        DELIVERY_TRACKER[Delivery Tracker]
    end
    
    SUBMIT_QUEUE --> APPROVAL_ENGINE
    APPROVAL_ENGINE --> AUDIT_LOGGER
    APPROVAL_ENGINE --> LOCK_VALIDATOR
    APPROVAL_ENGINE --> NOTIFICATION_DISPATCHER
    
    NOTIFICATION_DISPATCHER --> EMAIL_COLLECTOR
    EMAIL_COLLECTOR --> QUEUE_PROCESSOR
    QUEUE_PROCESSOR --> DELIVERY_TRACKER
    
    LOCK_VALIDATOR --> IMMUTABLE_STORE
    LOCK_VALIDATOR --> LOCK_MONITOR
```

### 16.3 Database Architecture Enhancements

#### 16.3.1 Approval Status Tracking
```sql
-- Enhanced enrollments table
ALTER TABLE enrollments ADD COLUMN admin_approval_status VARCHAR(20) DEFAULT 'pending';
ALTER TABLE enrollments ADD COLUMN approved_at TIMESTAMP;
ALTER TABLE enrollments ADD COLUMN approved_by UUID REFERENCES profiles(id);
ALTER TABLE enrollments ADD COLUMN approval_notes TEXT;
ALTER TABLE enrollments ADD COLUMN forms_locked_at TIMESTAMP;

-- Approval audit trail
CREATE TABLE enrollment_approval_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) NOT NULL,
    enrollment_id UUID REFERENCES enrollments(id) NOT NULL,
    admin_id UUID REFERENCES profiles(id) NOT NULL,
    action VARCHAR(20) NOT NULL,
    previous_status VARCHAR(20),
    new_status VARCHAR(20) NOT NULL,
    notes TEXT,
    affected_forms JSONB DEFAULT '[]',
    created_at TIMESTAMP DEFAULT NOW()
);
```

### 16.4 API Service Architecture

#### 16.4.1 Admin Approval Handler
```rust
// Lambda handler structure
pub struct AdminApprovalHandler {
    db_client: Arc<DatabaseClient>,
    audit_service: Arc<AuditService>,
    notification_service: Arc<NotificationService>,
    form_lock_service: Arc<FormLockService>,
}

impl AdminApprovalHandler {
    pub async fn approve_enrollment(&self, enrollment_id: Uuid, admin_id: Uuid, notes: Option<String>) -> Result<ApprovalResponse> {
        // 1. Validate admin permissions
        // 2. Check all required forms completed
        // 3. Update enrollment status
        // 4. Lock all forms
        // 5. Create audit record
        // 6. Trigger notifications
        // 7. Update real-time subscriptions
    }
    
    pub async fn request_revision(&self, enrollment_id: Uuid, admin_id: Uuid, revision_request: RevisionRequest) -> Result<RevisionResponse> {
        // 1. Update enrollment status to 'needs_revision'
        // 2. Unlock specific forms if specified
        // 3. Create audit record with affected forms
        // 4. Queue revision request notifications
    }
}
```

### 16.5 Form Locking Mechanism

#### 16.5.1 Lock Validation Flow
```mermaid
sequenceDiagram
    participant Parent
    participant FormWebhook
    participant LockValidator
    participant Database
    participant AuditLog
    
    Parent->>FormWebhook: Submit form data
    FormWebhook->>LockValidator: Validate submission allowed
    LockValidator->>Database: Check enrollment approval status
    Database-->>LockValidator: Return status & lock info
    
    alt Forms are locked
        LockValidator-->>FormWebhook: Reject: Forms locked
        FormWebhook-->>Parent: Error: Cannot modify locked forms
        FormWebhook->>AuditLog: Log blocked submission attempt
    else Forms unlocked
        LockValidator-->>FormWebhook: Allow submission
        FormWebhook->>Database: Process form submission
        Database-->>FormWebhook: Submission saved
        FormWebhook-->>Parent: Success response
    end
```

### 16.6 Real-time Updates Architecture

#### 16.6.1 Approval Event Broadcasting
```rust
// Supabase Realtime integration
pub async fn broadcast_approval_event(
    supabase_client: &SupabaseClient,
    enrollment_id: Uuid,
    event_type: ApprovalEventType,
    event_data: ApprovalEventData,
) -> Result<()> {
    let channel = format!("enrollment:{}", enrollment_id);
    
    supabase_client
        .realtime()
        .channel(&channel)
        .send(json!({
            "event": event_type.to_string(),
            "payload": event_data,
            "timestamp": Utc::now(),
        }))
        .await?;
        
    Ok(())
}
```

### 16.7 Notification Architecture

#### 16.7.1 Multi-Email Processing Pipeline
```mermaid
graph LR
    subgraph "Notification Trigger"
        APPROVAL_ACTION[Admin Action]
        STATUS_CHANGE[Status Change Event]
    end
    
    subgraph "Email Collection"
        GET_PRIMARY[Get Primary Email]
        GET_ADDITIONAL[Get Additional Emails]
        MERGE_LISTS[Merge & Deduplicate]
    end
    
    subgraph "Queue Processing"
        QUEUE_BATCH[Batch Queue Items]
        SEND_EMAILS[Send via Resend API]
        TRACK_DELIVERY[Track Delivery Status]
    end
    
    APPROVAL_ACTION --> STATUS_CHANGE
    STATUS_CHANGE --> GET_PRIMARY
    STATUS_CHANGE --> GET_ADDITIONAL
    GET_PRIMARY --> MERGE_LISTS
    GET_ADDITIONAL --> MERGE_LISTS
    MERGE_LISTS --> QUEUE_BATCH
    QUEUE_BATCH --> SEND_EMAILS
    SEND_EMAILS --> TRACK_DELIVERY
```

### 16.8 Security & Compliance

#### 16.8.1 Approval Authorization Matrix
| Role | View Enrollments | Approve/Reject | Request Revision | View Audit Trail |
|------|------------------|----------------|------------------|------------------|
| **Parent** | Own only | ❌ | ❌ | Own only |
| **Teacher** | Assigned classes | ❌ | ❌ | ❌ |
| **Admin** | School-wide | ✅ | ✅ | School-wide |
| **Super Admin** | All schools | ✅ | ✅ | All schools |

#### 16.8.2 Data Protection
- **Form Lock Immutability**: Approved forms cannot be unlocked except via official revision requests
- **Audit Compliance**: Complete history of all approval actions with timestamps and admin identification
- **Multi-Email Privacy**: Additional email addresses follow same COPPA/FERPA protections
- **Real-time Security**: All broadcast channels require authentication and school context

### 16.9 Performance Considerations

#### 16.9.1 Optimization Strategies
- **Database Functions**: Complex approval logic implemented as stored procedures
- **Connection Pooling**: Shared database connections across Lambda functions
- **Async Processing**: Non-blocking notification queue processing
- **Caching**: Approval status cached in Redis for high-frequency checks
- **Batch Operations**: Multiple email notifications processed in batches

### 16.10 Monitoring & Alerting

#### 16.10.1 Key Metrics
- **Approval Processing Time**: Average time from submission to admin decision
- **Form Lock Compliance**: Percentage of blocked unauthorized edit attempts
- **Multi-Email Delivery Rate**: Success rate of notification delivery to all parent emails
- **API Response Times**: Performance metrics for approval endpoints
- **Queue Processing**: Notification queue depth and processing times

## 17. Conclusion

This architecture design provides a robust, scalable, and secure foundation for The Goddard School Enrollment Management System. The combination of React.js, Rust with AWS Lambda, and Supabase offers:

### ✅ **Strengths**
- **Performance**: 6x faster than traditional Python/Node.js backends
- **Cost-effective**: Serverless pay-per-use model with ARM64 optimization
- **Scalable**: Auto-scaling with no infrastructure management
- **Secure**: Multiple security layers with RLS and compliance features
- **Modern**: Real-time capabilities and excellent developer experience

### ⚠️ **Considerations**
- **Cold starts**: Mitigated with provisioned concurrency
- **Rust learning curve**: Offset by long-term performance benefits
- **Vendor dependencies**: Balanced with portable architecture patterns

The architecture is designed to evolve from a focused enrollment system to a comprehensive school management platform, with clear separation of concerns and well-defined boundaries that facilitate future enhancements.

---

## Appendix A: Technology Stack Summary (Updated)

| Layer | Technology | Version | Purpose | Scale Justification |
|-------|------------|---------|---------|---------------------|
| **Frontend** | React.js | 18.x | UI Framework | Handles 500 concurrent users easily |
| | TypeScript | 5.x | Type Safety | Developer productivity |
| | Vite | 5.x | Build Tool | Fast builds for CI/CD |
| | Tailwind CSS | 3.x | Styling | Component-based styling |
| | React Query | 5.x | Data Fetching | Caching for performance |
| **Hosting** | Cloudflare Pages | - | Static Hosting | Global CDN, team familiarity |
| **Forms** | Fillout.com | Business | Form Builder | 10+ forms with 3-tier assignment hierarchy |
| **Email** | Resend | - | Email Service | Developer-friendly API |
| **Backend** | Rust | 1.75+ | Language | Performance for 500 concurrent |
| | AWS Lambda | - | Compute | Auto-scales to demand |
| | Axum | 0.7 | Web Framework | Efficient HTTP handling |
| **Database** | Supabase | Pro | BaaS Platform | Includes auth, realtime, storage |
| | PostgreSQL | 15.x | Database | Handles multi-tenant load |
| **Auth** | Supabase Auth | - | Authentication | Integrated with RLS |
| **Infrastructure** | AWS + Cloudflare | - | Hybrid Cloud | Best of both platforms |
| | Terraform | 1.6 | IaC | Infrastructure management |
| | GitHub Actions | - | CI/CD | Dual deployment automation |

## Appendix B: Security Compliance Checklist

- [ ] COPPA compliance implementation
- [ ] FERPA compliance implementation
- [ ] PII encryption at rest
- [ ] PII encryption in transit
- [ ] Audit logging for all data access
- [ ] Data retention policies
- [ ] Right to deletion (GDPR-ready)
- [ ] Security incident response plan
- [ ] Regular security audits
- [ ] Penetration testing schedule

---

*This document represents the initial high-level design and will evolve based on feedback, prototyping results, and changing requirements.*