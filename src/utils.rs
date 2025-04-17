use std::time::{Duration, Instant};

/**
* filename : utils
* author : HAMA
* date: 2025. 4. 17.
* description: 
**/


// ----------- retryer 사용예 ----------------

//   let policy = RetryPolicy::new(5, |retry_count| {
//     match retry_count {
//       1 => 1000,        // 첫 번째 재시도: 1초 대기
//       n if n < 3 => 2000, // 2-3번째 재시도: 2초 대기
//       n if n < 5 => 5000, // 4-5번째 재시도: 5초 대기
//       _ => 10000,       // 그 이후: 10초 대기
//     }
//   });
//
// let result = retryer.execute(|| async {
//   call_openai_server_api(query_for_failure).await
// }).await;


// 서킷 브레이커 상태
#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
  Closed,     // 정상 작동 - API 호출 허용
  Open,       // 에러 발생 - API 호출 차단
  HalfOpen,   // 테스트 단계 - 제한적 API 호출 허용
}

// 서킷 브레이커 구조체
struct CircuitBreaker {
  state: CircuitState,
  failure_count: u32,
  failure_threshold: u32,     // 이 횟수 이상 실패하면 circuit open
  reset_timeout_ms: u64,      // circuit을 half-open 상태로 전환하기까지의 시간
  last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
  fn new(failure_threshold: u32, reset_timeout_ms: u64) -> Self {
    CircuitBreaker {
      state: CircuitState::Closed,
      failure_count: 0,
      failure_threshold,
      reset_timeout_ms,
      last_failure_time: None,
    }
  }
  
  fn record_success(&mut self) {
    self.failure_count = 0;
    self.state = CircuitState::Closed;
    println!("🔄 Circuit breaker reset to CLOSED state after success");
  }
  
  fn record_failure(&mut self) {
    self.failure_count += 1;
    self.last_failure_time = Some(Instant::now());
    
    if self.failure_count >= self.failure_threshold {
      self.state = CircuitState::Open;
      println!("🚫 Circuit OPENED after {} consecutive failures", self.failure_count);
    }
  }
  
  fn can_execute(&mut self) -> bool {
    match self.state {
      CircuitState::Closed => true,
      CircuitState::Open => {
        // reset_timeout 이후에는 HalfOpen 상태로 전환하여 시도해봄
        if let Some(failure_time) = self.last_failure_time {
          if failure_time.elapsed() >= Duration::from_millis(self.reset_timeout_ms) {
            println!("🔍 Circuit changed from OPEN to HALF-OPEN. Will try one request.");
            self.state = CircuitState::HalfOpen;
            return true;
          }
        }
        let remaining_ms = if let Some(failure_time) = self.last_failure_time {
          let elapsed_ms = failure_time.elapsed().as_millis() as u64;
          if elapsed_ms < self.reset_timeout_ms {
            self.reset_timeout_ms - elapsed_ms
          } else {
            0
          }
        } else {
          0
        };
        println!("🚫 Circuit is OPEN. Blocking request. Will try again in {} ms", remaining_ms);
        false
      },
      CircuitState::HalfOpen => true,
    }
  }
}

// 재시도 정책을 정의하는 구조체
struct RetryPolicy<F>
where
  F: Fn(u32) -> u64,
{
  max_retries: u32,
  backoff_ms: F,
}

impl<F> RetryPolicy<F>
where
  F: Fn(u32) -> u64,
{
  // 새로운 재시도 정책 생성
  fn new(max_retries: u32, backoff_ms: F) -> Self {
    RetryPolicy {
      max_retries,
      backoff_ms,
    }
  }
  
  // 현재 시도 횟수에 따른 대기 시간 반환
  fn get_backoff_ms(&self, retry_count: u32) -> u64 {
    (self.backoff_ms)(retry_count)
  }
}

// Retryer 구조체 정의
struct Retryer<BP>
where
  BP: Fn(u32) -> u64,
{
  policy: RetryPolicy<BP>,
  circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

impl<BP> Retryer<BP>
where
  BP: Fn(u32) -> u64,
{
  // 새로운 Retryer 인스턴스 생성
  fn new(
    max_retries: u32,
    backoff_function: BP,
    failure_threshold: u32,
    reset_timeout_ms: u64,
  ) -> Self {
    let policy = RetryPolicy::new(max_retries, backoff_function);
    let circuit_breaker = Arc::new(Mutex::new(
      CircuitBreaker::new(failure_threshold, reset_timeout_ms)
    ));
    
    Retryer {
      policy,
      circuit_breaker,
    }
  }
  
  // 직접 RetryPolicy와 CircuitBreaker 인스턴스로 생성
  fn with_components(
    policy: RetryPolicy<BP>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
  ) -> Self {
    Retryer {
      policy,
      circuit_breaker,
    }
  }
  
  // 작업 실행 및 재시도 로직
  async fn execute<T, E, F, Fut>(&self, operation: F) -> Result<T, E>
  where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Display,
  {
    let mut retry_count = 0;
    
    loop {
      // 서킷 브레이커 확인
      {
        let mut breaker = self.circuit_breaker.lock().unwrap();
        if !breaker.can_execute() {
          // 서킷이 열려있으면 바로 에러 반환
          return operation().await; // 실패할 것이지만 에러 타입을 맞추기 위해 한 번 호출
        }
      }
      
      // 작업 실행
      match operation().await {
        Ok(result) => {
          // 성공 시 서킷 브레이커 리셋
          let mut breaker = self.circuit_breaker.lock().unwrap();
          breaker.record_success();
          return Ok(result);
        }
        Err(err) => {
          // 실패 시 서킷 브레이커 업데이트
          {
            let mut breaker = self.circuit_breaker.lock().unwrap();
            breaker.record_failure();
          }
          
          retry_count += 1;
          
          if retry_count >= self.policy.max_retries {
            println!("🛑 Maximum retry attempts ({}) reached. Giving up.", self.policy.max_retries);
            return Err(err);
          }
          
          let wait_time_ms = self.policy.get_backoff_ms(retry_count);
          
          println!("⏱️  Retry attempt {}/{}. Waiting for {} ms before next attempt...",
                   retry_count, self.policy.max_retries, wait_time_ms);
          println!("   Last error: {}", err);
          
          // 대기 시간 동안 대기
          tokio::time::sleep(Duration::from_millis(wait_time_ms)).await;
        }
      }
    }
  }
  
  // 서킷 브레이커 상태 반환 메서드
  fn get_circuit_state(&self) -> CircuitState {
    let breaker = self.circuit_breaker.lock().unwrap();
    breaker.state.clone()
  }
  
  // 실패 카운트 반환 메서드
  fn get_failure_count(&self) -> u32 {
    let breaker = self.circuit_breaker.lock().unwrap();
    breaker.failure_count
  }
}