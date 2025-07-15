// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2025 rtldg <rtldg@protonmail.com>

#include "../../../srcwrtimer/extshared/src/extension.h"
#include "../../../srcwrtimer/extshared/src/coreident.hpp"

#include <ICellArray.h>


extern "C" {

void rust_setup_replay_thread();
void rust_KILL_replay_thread();

void rust_post_to_replay_thread(
	// bitch
	  const char* replayfmt
	, int replaysubversion
	//
	, IChangeableForward* forward
	// what to pass along to the callback
	, int value
	// blahlahalkahlalkhlk
	, const char* replayFolderOrig
	, const char* replayFolder
	, const char* map
	// copy of SaveReplay() params
	, int style
	, int track
	, float time
	, int steamid
	, int preframes
	, void* playerrecording
	, int iSize
	, int postframes
	, int timestamp
	, float* fZoneOffset
	, bool saveCopy
	, bool saveWR
	, float tickrate
);

}


extern const sp_nativeinfo_t FloppyNatives[];


void MyExtension::OnHandleDestroy(HandleType_t type, void* object) {}
bool MyExtension::GetHandleApproxSize(HandleType_t type, void* object, unsigned int* size) { return false; }


bool Extension_OnLoad(char* error, size_t maxlength)
{
	rust_setup_replay_thread();

	sharesys->AddNatives(myself, FloppyNatives);
	return true;
}

void Extension_OnUnload()
{
	rust_KILL_replay_thread();
}

void Extension_OnAllLoaded() {}

static cell_t N_SRCWRFloppy_AsyncSaveReplay(IPluginContext* ctx, const cell_t* params)
{
	char *replayfmt;
	(void)ctx->LocalToString(params[1], &replayfmt);
	int replaysubversion = params[2];

	if (0 != strcmp(replayfmt, "{SHAVITREPLAYFORMAT}{FINAL}") || replaysubversion != 0x09)
	{
		// TODO: Throw error
	}

	cell_t callback = params[3];
	int value = params[4];

	char *replayFolderOrig, replayFolder[PLATFORM_MAX_PATH];
	(void)ctx->LocalToString(params[5], &replayFolderOrig);
	smutils->BuildPath(Path_SM, replayFolder, sizeof(replayFolder), "%s", replayFolderOrig);
	char *map;
	(void)ctx->LocalToString(params[6], &replayFolderOrig);

	int style = params[7];
	int track = params[8];
	float time = params[9];
	int steamid = params[10];
	int preframes = params[11];
	ICellArray *arraylist;
	Handle_t playerrecording = params[12];
	if (HandleError err = ReadHandleCoreIdent(playerrecording, g_ArrayListType, (void **)&arraylist); err != HandleError_None)
		return ctx->ThrowNativeError("Invalid ArrayList Handle %x (error %d)", playerrecording, err);
	int iSize = params[13];
	int postframes = params[14];
	int timestamp = params[15];
	float* fZoneOffset;
	if (SP_ERROR_NONE != ctx->LocalToPhysAddr(params[16], (cell_t**)&fZoneOffset))
		return ctx->ThrowNativeError("fuck");
	bool saveCopy = params[17];
	bool saveWR = params[18];
	float tickrate = params[19];

	IChangeableForward* fw = forwards->CreateForwardEx(
		  NULL
		, ET_Ignore
		, 3
		, NULL
		, Param_Any // saved
		, Param_Any // value
		, Param_String // sPath
	);
	if (!fw)
		return ctx->ThrowNativeError("failed to create a forward...");
	if (!fw->AddFunction(ctx, callback))
	{
		forwards->ReleaseForward(fw);
		return ctx->ThrowNativeError("Failed to add callback to forward thing...");
	}

	rust_post_to_replay_thread(
		  replayfmt
		, replaysubversion
		, fw
		, value
		, replayFolderOrig
		, replayFolder
		, map
		, style
		, track
		, time
		, steamid
		, preframes
		, arraylist
		, iSize
		, postframes
		, timestamp
		, fZoneOffset
		, saveCopy
		, saveWR
		, tickrate
	);

	return 0; // native marked as void so return value doesn't matter...
}

extern const sp_nativeinfo_t FloppyNatives[] = {
	{"SRCWRFloppy_AsyncSaveReplay", N_SRCWRFloppy_AsyncSaveReplay},
	{NULL, NULL}
};
