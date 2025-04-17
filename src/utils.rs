use std::time::{Duration, Instant};

/**
* filename : utils
* author : HAMA
* date: 2025. 4. 17.
* description: 
**/


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


// 지정된 정책에 따라 재시도하는 함수
async fn retry_with_policy<T, E, F, Fut, BP>(
  operation: F,
  policy: RetryPolicy<BP>,
) -> Result<T, E>
where
  F: Fn() -> Fut,
  Fut: std::future::Future<Output = Result<T, E>>,
  E: std::fmt::Display,
  BP: Fn(u32) -> u64,
{
  let mut retry_count = 0;
  
  loop {
    // 작업 실행
    match operation().await {
      Ok(result) => {
        return Ok(result);
      }
      Err(err) => {
        retry_count += 1;
        
        if retry_count >= policy.max_retries {
          println!("🛑 Maximum retry attempts ({}) reached. Giving up.", policy.max_retries);
          return Err(err);
        }
        
        let wait_time_ms = policy.get_backoff_ms(retry_count);
        
        println!("⏱️  Retry attempt {}/{}. Waiting for {} ms before next attempt...",
                 retry_count, policy.max_retries, wait_time_ms);
        println!("   Last error: {}", err);
        
        // 대기 시간 동안 대기
        tokio::time::sleep(Duration::from_millis(wait_time_ms)).await;
      }
    }
  }
}

// ----------- retry 사용예 ----------------

//   let policy = RetryPolicy::new(5, |retry_count| {
//     match retry_count {
//       1 => 1000,        // 첫 번째 재시도: 1초 대기
//       n if n < 3 => 2000, // 2-3번째 재시도: 2초 대기
//       n if n < 5 => 5000, // 4-5번째 재시도: 5초 대기
//       _ => 10000,       // 그 이후: 10초 대기
//     }
//   });
//
//   let result = retry_with_policy(
//     || async { call_openai_server_api(query_for_failure).await },
//     policy,
//   ).await;




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
    println!("Circuit breaker reset to CLOSED state after success");
  }
  
  fn record_failure(&mut self) {
    self.failure_count += 1;
    self.last_failure_time = Some(Instant::now());
    
    if self.failure_count >= self.failure_threshold {
      self.state = CircuitState::Open;
      println!("Circuit OPENED after {} consecutive failures", self.failure_count);
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
        println!(" Circuit is OPEN. Blocking request. Will try again in {} ms", remaining_ms);
        false
      },
      CircuitState::HalfOpen => true,
    }
  }
}