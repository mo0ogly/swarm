//go:build linux && !amd64

package engine

func runKernelObserveNative(p TrustKernelObservePolicy, l kernelObservationLease, _ string, _ func() (kernelObservationLease, error), _ func(kernelObservation) error) (TrustKernelObserveResult, error) {
	r := kernelObserveFixed(p, l)
	r.StopReason = "unavailable"
	r.Diagnostic = "architecture unsupported"
	r.CleanupVerified = true
	return r, nil
}
