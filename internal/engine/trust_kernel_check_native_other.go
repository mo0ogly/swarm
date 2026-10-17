//go:build linux && !amd64

package engine

import "os"

func runKernelOwnedFixture(p TrustKernelCheckPolicy, _ string, _ string) TrustKernelCheckResult {
	r := kernelCheckFixedResult()
	r.CleanupVerified = true
	r.EffectivePolicy = p
	r.OwnedPIDs = []int{os.Getpid()}
	r.Reason = "architecture_unsupported"
	return r
}
