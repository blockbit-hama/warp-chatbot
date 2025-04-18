use std::fmt::Display;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rand::Rng;
use tokio::time::sleep;

/**
* filename : utils
* author : HAMA
* date: 2025. 4. 17.
* description: 
**/



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

// 재시도 실행 함수
async fn retry_async<FN, Fut, T, E, B>(
  policy: &RetryPolicy<B>,
  mut operation: FN,
) -> Result<T, E>
where
  FN: FnMut() -> Fut,
  Fut: std::future::Future<Output = Result<T, E>>,
  E: Display,
  B: Fn(u32) -> u64,
{
  let mut attempt = 0;
  
  loop {
    let result = operation().await;
    
    match result {
      Ok(value) => return Ok(value),
      Err(e) => {
        attempt += 1;
        if attempt >= policy.max_retries {
          println!(" 재시도 초과. 마지막 에러: {}", e);
          return Err(e);
        }
        
        let base_backoff = policy.get_backoff_ms(attempt);
        let jitter = rand::thread_rng().gen_range(0..=100);
        let delay = base_backoff + jitter;
        
        println!(
          "{}번째 재시도 실패. {}ms 후 재시도합니다. 에러: {}",
          attempt, delay, e
        );
        
        sleep(Duration::from_millis(delay)).await;
      }
    }
  }
}



// ----------- retryer 사용예 ----------------
// CircuitBreaker 는 전역적으로 Retry는 독립적으로

//   let policy = RetryPolicy::new(5, |retry_count| {
//     match retry_count {
//       1 => 1000,        // 첫 번째 재시도: 1초 대기
//       n if n < 3 => 2000, // 2-3번째 재시도: 2초 대기
//       n if n < 5 => 5000, // 4-5번째 재시도: 5초 대기
//       _ => 10000,       // 그 이후: 10초 대기
//     }
//   });

//
// // 테스트용 API 호출 함수
// async fn call_external_api() -> Result<String, String> {
//   let success = rand::thread_rng().gen_bool(0.4);
//   if success {
//     Ok("외부 API 응답 성공".into())
//   } else {
//     Err("외부 API 에러".into())
//   }
// }
//
// // 실제 처리 함수
// async fn handle_request(breaker: Arc<Mutex<CircuitBreaker>>) {
//   {
//     let mut br = breaker.lock().unwrap();
//     if !br.can_execute() {
//       println!("🚫 Circuit open: 요청 차단됨");
//       return;
//     }
//   }
//
//   let result = retry_async(3, call_external_api).await;
//
//   match result {
//     Ok(msg) => {
//       println!("{msg}");
//       breaker.lock().unwrap().record_success();
//     }
//     Err(e) => {
//       println!("🔥 최종 실패: {e}");
//       breaker.lock().unwrap().record_failure();
//     }
//   }
// }
//
// #[tokio::main]
// async fn main() {
//   let breaker = Arc::new(Mutex::new(CircuitBreaker::new(3, Duration::from_secs(10))));
//
//   for _ in 0..10 {
//     let br = breaker.clone();
//     task::spawn(async move {
//       handle_request(br).await;
//     });
//     sleep(Duration::from_millis(500)).await;
//   }
// }