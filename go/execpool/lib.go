package execpool

import (
	"context"
	"fmt"
	"sync"
	"sync/atomic"
	"time"
)

type Task struct {
	ctx    context.Context
	cancel context.CancelFunc
	fn     func(ctx context.Context)
}

type ExecPool struct {
	taskChan chan Task
	wg       sync.WaitGroup

	ctx    context.Context
	cancel context.CancelFunc

	closed atomic.Bool
}

func NewExecPool(maxWorkers int, queueSize int) *ExecPool {
	return &ExecPool{
		taskChan: make(chan Task, queueSize),
	}
}

func (p *ExecPool) Submit(timeout time.Duration, fn func(context.Context)) {
	if p.closed.Load() {
		fmt.Println("Already stopped")
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), timeout)

	task := Task{
		ctx:    ctx,
		cancel: cancel,
		fn:     fn,
	}

	select {
	case p.taskChan <- task:
	default:
		fmt.Println("Queue already full, abandon task")
		cancel()
	}
}

func (p *ExecPool) Start(workers int) {
	for i := 0; i < workers; i += 1 {
		p.wg.Add(1)
		go func() {
			defer p.wg.Done()
			for task := range p.taskChan {
				select {
				case <-task.ctx.Done():
					fmt.Println("Timeout at ExecPool")
					task.cancel()
					continue
				default:
				}

				func() {
					defer func() {
						if r := recover(); r != nil {
							fmt.Printf("panic")
						}
						task.cancel()
					}()
					task.fn(task.ctx)
				}()
			}
		}()
	}
}

func (p *ExecPool) Stop() {
	if !p.closed.CompareAndSwap(false, true) {
		return
	}

	close(p.taskChan)
	p.wg.Wait()
}
