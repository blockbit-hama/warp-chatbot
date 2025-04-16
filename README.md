# warp-chatbot 프로젝트

Rust 기반의 고성능 웹 애플리케이션 프레임워크 **Warp**으로 제작된 채팅 서비스 플랫폼. REST API, 로깅/추적 시스템, 데이터베이스 연동, 인증 메커니즘, 서드파티 통합 및 배포 인프라를 포함한 완전한 기능을 갖춘 백엔드 시스템.

## 📦 현재 구현 기능
- **Warp Framework**: 비동기 런타임 기반 고성능 REST API
- **Observability**: OpenTelemetry 연동 로깅/트레이싱
- **데이터베이스**: PostgreSQL + Diesel ORM
- **인증**: JWT 기반 권한 관리 시스템
- **서드파티**: Slack/Google API 연동 모듈
- **배포**: Docker+Kubernetes 구성파일 포함

---

## 🚀 RAG 기반 챗봇 기능 추가 계획

### 1. 핵심 컴포넌트 구성
// RAG 처리 파이프라인 예시
struct RagPipeline {
retriever: Arc<dyn Retriever>,
llm: Arc<dyn LanguageModel>,
doc_processor: DocumentProcessor
}


### 2. 필요 기술 스택
| 분류 | 도구 | 선택 이유 |
|-------|------|----------|
| 벡터 DB | **Pinecone** | 실시간 임베딩 검색 최적화 |
| LLM API | **Anthropic Claude 3** | 한국어 처리 우수성 |
| 프레임워크 | **LlamaIndex** | 다중 데이터 소스 연결 |

### 3. 구현 단계
1. **문서 수집 계층**
    - Google Drive/Notion 콘텐츠 크롤러 구현
    - 자동 동기화 메커니즘 (Polling+Webhook)

2. **임베딩 처리**
    - `sentence-transformers/all-MiniLM-L6-v2` 모델 활용
    - 384차원 벡터 변환 파이프라인

3. **검색 향상 기능**
   hybrid_search:
   semantic_weight: 0.7
   keyword_weight: 0.3
   filter:
- source_type: [pdf, md]
- updated_at: ">2025-01-01"



---

## 🛠 설치 및 실행
벡터 DB 설정 (Pinecone 예시)
export PINECONE_API_KEY="your-key"
export PINECONE_ENV="gcp-starter"

LLM 클라이언트 구성
cargo add anthropic-rs --features async


---

## 📈 향후 개선 방안
1. 자동 재학습 파이프라인 구축
2. 크로스-리전 백업 메커니즘
3. 실시간 사용자 피드백 반영 시스템