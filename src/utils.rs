use std::time::Duration;

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


// example
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//   // 사용자 정의 재시도 정책 생성
//   let policy = RetryPolicy::new(5, |retry_count| {
//     match retry_count {
//       1 => 1000,        // 첫 번째 재시도: 1초 대기
//       n if n < 3 => 2000, // 2-3번째 재시도: 2초 대기
//       n if n < 5 => 5000, // 4-5번째 재시도: 5초 대기
//       _ => 10000,       // 그 이후: 10초 대기
//     }
//   });
//
//   println!("=== 실패 후 재시도 테스트 (실패 예상) ===");
//   // 실패할 쿼리로 테스트
//   let query_for_failure = "paul";
//   let result = retry_with_policy(
//     || async { call_openai_server_api(query_for_failure).await },
//     policy,
//   ).await;
//
//   match result {
//     Ok(response) => println!("최종 성공: {}", response),
//     Err(err) => println!("최종 실패: {}", err),
//   }
//
//   // 새 정책 생성 (성공 테스트용)
//   let success_policy = RetryPolicy::new(3, |retry_count| {
//     match retry_count {
//       1 => 1000,
//       _ => 2000,
//     }
//   });
//
//   println!("\n=== 성공 케이스 테스트 ===");
//   // 성공할 쿼리로 테스트
//   let query_for_success = "toto";
//   let result = retry_with_policy(
//     || async { call_openai_server_api(query_for_success).await },
//     success_policy,
//   ).await;
//
//   match result {
//     Ok(response) => println!("최종 성공: {}", response),
//     Err(err) => println!("최종 실패: {}", err),
//   }
//
//   // 복잡한 백오프 정책 예시
//   let advanced_policy = RetryPolicy::new(10, |retry_count| {
//     // 지수 백오프 + 최대 제한
//     let base = 1000;  // 기본 1초
//     let exp_backoff = base * 2u64.pow(retry_count - 1);  // 지수적으로 증가
//     std::cmp::min(exp_backoff, 30000)  // 최대 30초로 제한
//   });
//
//   println!("\n=== 고급 정책 테스트 ===");
//   let result = retry_with_policy(
//     || async { call_openai_server_api("will_fail").await },
//     advanced_policy,
//   ).await;
//
//   match result {
//     Ok(response) => println!("최종 성공: {}", response),
//     Err(err) => println!("최종 실패: {}", err),
//   }
//
//   Ok(())
// }