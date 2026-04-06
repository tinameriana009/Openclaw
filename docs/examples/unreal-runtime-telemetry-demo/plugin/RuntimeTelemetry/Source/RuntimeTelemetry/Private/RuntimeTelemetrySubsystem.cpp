#include "RuntimeTelemetrySubsystem.h"

#include "Logging/LogMacros.h"

DEFINE_LOG_CATEGORY_STATIC(LogRuntimeTelemetry, Log, All);

void URuntimeTelemetrySubsystem::RecordEvent(const FString& EventName, const FString& Payload)
{
    FRuntimeTelemetryEvent& NewEvent = BufferedEvents.AddDefaulted_GetRef();
    NewEvent.Name = EventName;
    NewEvent.Payload = Payload;

    UE_LOG(LogRuntimeTelemetry, Verbose, TEXT("Buffered telemetry event: %s"), *EventName);
}

void URuntimeTelemetrySubsystem::FlushEvents()
{
    UE_LOG(LogRuntimeTelemetry, Display, TEXT("Flushing %d telemetry events"), BufferedEvents.Num());

    // Intentionally conservative demo behavior: clear the in-memory buffer.
    // Real export paths and durability guarantees need project-specific validation.
    BufferedEvents.Reset();
}

int32 URuntimeTelemetrySubsystem::GetBufferedEventCount() const
{
    return BufferedEvents.Num();
}
